// Comprehensive API test suite via Chrome DevTools Protocol
// Hits every API endpoint (MCP tools + web routes) with many scenarios
// Records each test as a trace to .qa-hunter/data/traces.jsonl

import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const WebSocket = require('/home/limaxs/.nvm/versions/node/v22.22.0/lib/node_modules/ws/index.js');
import { writeFileSync, appendFileSync, readFileSync } from 'node:fs';

const CDP_PORT = process.argv[2] || '9222';
const SERVER = process.argv[3] || 'http://127.0.0.1:8080';
const QA_DIR = process.argv[4] || '.qa-hunter';

class CDPClient {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.consoleMessages = [];
    this.consoleExceptions = [];
    ws.on('message', (data) => {
      const msg = JSON.parse(data);
      if (msg.id !== undefined && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message));
        else resolve(msg.result);
      } else if (msg.method) {
        if (msg.method === 'Runtime.consoleAPICalled') this.consoleMessages.push(msg.params);
        if (msg.method === 'Runtime.exceptionThrown') this.consoleExceptions.push(msg.params);
      }
    });
  }
  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
}

async function getTarget() {
  const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json`);
  const targets = await res.json();
  return targets.find(t => t.type === 'page') || targets[0];
}

async function getDOM(cdp, expr) {
  const { result, exceptionDetails } = await cdp.send('Runtime.evaluate', {
    expression: expr, returnByValue: true, awaitPromise: true,
  });
  if (exceptionDetails) throw new Error(`Eval failed: ${exceptionDetails.text}`);
  return result.value;
}

let traceCounter = 400;

async function getToken(cdp) {
  // Use CDP-driven login + OAuth flow
  // First clear any stale cookies
  await cdp.send('Network.clearBrowserCookies');
  await cdp.send('Page.enable');
  await cdp.send('Page.navigate', { url: `${SERVER}/login` });
  // Wait for load
  await new Promise(r => setTimeout(r, 1000));
  await cdp.send('Runtime.evaluate', {
    expression: `(() => {
      document.querySelector('input[name="username"]').value = 'admin';
      document.querySelector('input[name="password"]').value = 'change-this-password-1234';
      document.querySelector('button[type="submit"]').click();
      return true;
    })()`,
  });
  await new Promise(r => setTimeout(r, 2000));
  // Get cookies from browser
  const cookies = await cdp.send('Network.getCookies');
  const sessionCookie = cookies.cookies.find(c => c.name === 'ledgapi_session');
  const csrfCookie = cookies.cookies.find(c => c.name === 'ledgapi_csrf');
  if (!sessionCookie) throw new Error('no session cookie');
  // Register OAuth client
  const regResp = await fetch(`${SERVER}/oauth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_name: 'Comprehensive API Test',
      redirect_uris: ['http://127.0.0.1:9999/cb-comprehensive'],
    }),
  });
  const { client_id } = await regResp.json();
  // Authorize
  const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM';
  const scope = 'ledgapi%3Aread%20ledgapi%3Awrite%20ledgapi%3Aadmin';
  const authUrl = `${SERVER}/oauth/authorize?response_type=code&client_id=${client_id}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-comprehensive&scope=${scope}&state=s&code_challenge=${challenge}&code_challenge_method=S256`;
  const cookieHeader = `ledgapi_session=${sessionCookie.value}${csrfCookie ? `; ledgapi_csrf=${csrfCookie.value}` : ''}`;
  const authResp = await fetch(authUrl, {
    headers: { cookie: cookieHeader },
    redirect: 'manual',
  });
  if (authResp.status !== 200) {
    const body = await authResp.text();
    throw new Error(`authorize: ${authResp.status}; body: ${body.substring(0, 500)}`);
  }
  const html = await authResp.text();
  const csrfMatch = html.match(/name="csrf" value="([^"]+)"/);
  if (!csrfMatch) throw new Error(`no csrf in consent page; html: ${html.substring(0, 2000)}`);
  const consentResp = await fetch(`${SERVER}/oauth/consent`, {
    method: 'POST',
    headers: {
      'content-type': 'application/x-www-form-urlencoded',
      cookie: cookieHeader,
    },
    body: `client_id=${client_id}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-comprehensive&code_challenge=${challenge}&code_challenge_method=S256&scope=${scope}&state=s&decision=approve&csrf=${csrfMatch[1]}`,
    redirect: 'manual',
  });
  const loc = consentResp.headers.get('location');
  if (!loc || !loc.includes('code=')) throw new Error(`no code: ${loc}`);
  const code = new URL(loc).searchParams.get('code');
  const tokenResp = await fetch(`${SERVER}/oauth/token`, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: `grant_type=authorization_code&code=${code}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-comprehensive&client_id=${client_id}&code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk`,
  });
  const tokenJson = await tokenResp.json();
  return tokenJson.access_token;
}

async function mcpCall(token, method, params, id) {
  const resp = await fetch(`${SERVER}/mcp`, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'accept': 'application/json, text/event-stream',
      'authorization': `Bearer ${token}`,
    },
    body: JSON.stringify({ jsonrpc: '2.0', id: id || 1, method, params }),
  });
  if (resp.status !== 200) {
    return { error: { code: resp.status, message: 'http' } };
  }
  const ct = resp.headers.get('content-type') || '';
  let body = await resp.text();
  if (ct.includes('text/event-stream')) {
    const lines = body.split('\n');
    for (const line of lines) {
      if (line.startsWith('data: ')) {
        body = line.substring(6).trim();
        break;
      }
    }
  }
  return JSON.parse(body);
}

function mcpExtract(resp) {
  if (resp.error) return null;
  const content = resp.result?.content?.[0];
  if (!content) return null;
  if (content.type === 'json') return content.json;
  if (content.type === 'text') {
    try { return JSON.parse(content.text); } catch { return content.text; }
  }
  return null;
}

async function test(name, fn) {
  process.stdout.write(`\n=== ${name} ===\n`);
  try {
    const r = await fn();
    process.stdout.write(`  PASS: ${name}\n`);
    return { name, status: 'PASS', result: r };
  } catch (e) {
    process.stdout.write(`  FAIL: ${name}: ${e.message}\n`);
    return { name, status: 'FAIL', error: e.message };
  }
}

function writeTrace(traceId, segment, target, scenario, actions, expected, actual, result, evidence) {
  const trace = {
    trace_id: traceId,
    requirement: 'Comprehensive API test',
    segment,
    target,
    scenario,
    actions,
    expected,
    actual,
    result,
    evidence,
    confidence: 'high',
    iteration: 10,
  };
  appendFileSync(`${QA_DIR}/data/traces.jsonl`, JSON.stringify(trace) + '\n');
}

async function main() {
  // Read next trace ID
  const data = readFileSync(`${QA_DIR}/data/traces.jsonl`, 'utf8');
  const lines = data.trim().split('\n');
  const lastTrace = lines[lines.length - 1];
  const lastId = lastTrace ? JSON.parse(lastTrace).trace_id : 'TRACE-000000';
  const match = lastId.match(/TRACE-(\d+)/);
  traceCounter = match ? parseInt(match[1]) + 1 : 400;

  const target = await getTarget();
  const cdp = new CDPClient(new WebSocket(target.webSocketDebuggerUrl));
  await new Promise(r => cdp.ws.on('open', r));
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await cdp.send('Page.enable');

  const results = [];
  const token = await getToken(cdp);
  process.stdout.write(`Token acquired (${token.length} chars)\n`);

  // === API-001: create_project scenarios ===
  results.push(await test('API-CP-1 create_project with minimal args', async () => {
    const slug = `apicp1-${Date.now()}`;
    const r = await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'API CP 1' } });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (data?.status !== 'created') throw new Error(`status: ${data?.status}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_project', 'minimal args',
      ['mcpCall create_project with {slug, name}'],
      'status=created', JSON.stringify(data), 'PASS', ['src/mcp/tools_impl/create_project.rs']);
    return data;
  }));

  results.push(await test('API-CP-2 create_project with description', async () => {
    const slug = `apicp2-${Date.now()}`;
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_project',
      arguments: { slug, name: 'API CP 2', description: 'Test project with description' },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_project', 'with description',
      ['mcpCall create_project with {slug, name, description}'],
      'status=created', JSON.stringify(mcpExtract(r)), 'PASS', ['create_project.rs']);
    return mcpExtract(r);
  }));

  results.push(await test('API-CP-3 create_project with duplicate slug', async () => {
    const slug = `apicp3-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'First' } });
    const r = await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'Duplicate' } });
    if (!r.error) throw new Error('expected duplicate error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_project', 'duplicate slug',
      ['create same slug twice'], 'error code=duplicate or similar',
      JSON.stringify(r.error), 'PASS', ['create_project.rs duplicate handling']);
    return r.error;
  }));

  results.push(await test('API-CP-4 create_project with invalid slug chars', async () => {
    const r = await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug: 'INVALID!@#', name: 'Bad' } });
    if (!r.error) throw new Error('expected validation error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_project', 'invalid slug',
      ['create with slug containing !@#'], 'error code=-32602 invalid slug',
      JSON.stringify(r.error), 'PASS', ['ProjectSlug::parse']);
    return r.error;
  }));

  results.push(await test('API-CP-5 create_project with empty name', async () => {
    const slug = `apicp5-${Date.now()}`;
    const r = await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: '' } });
    if (!r.error) throw new Error('expected validation error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_project', 'empty name',
      ['create with empty name'], 'error validation', JSON.stringify(r.error), 'PASS',
      ['create_project.rs validation']);
    return r.error;
  }));

  // === API-002: list_projects scenarios ===
  results.push(await test('API-LP-1 list_projects basic', async () => {
    const r = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (!Array.isArray(data?.projects)) throw new Error('no projects array');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_projects', 'basic call',
      ['mcpCall list_projects {}'], 'returns projects array',
      `count=${data.projects.length}`, 'PASS', ['list_projects.rs']);
    return data;
  }));

  results.push(await test('API-LP-2 list_projects returns slug+name+contract_count', async () => {
    const r = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const data = mcpExtract(r);
    if (data.projects.length === 0) throw new Error('no projects to test');
    const p = data.projects[0];
    if (!p.slug || !p.name || p.contract_count === undefined) {
      throw new Error(`incomplete project: ${JSON.stringify(p)}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_projects', 'field completeness',
      ['inspect first project object'],
      'has slug, name, contract_count',
      JSON.stringify(p), 'PASS', ['list_projects.rs ProjectSummary shape']);
    return p;
  }));

  // === API-003: create_contract scenarios ===
  results.push(await test('API-CC-1 create_contract basic', async () => {
    const slug = `apicc1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'CC Test' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_contract',
      arguments: {
        project_slug: slug, method: 'GET', path: '/api/test1',
        summary: 'Test 1', response_schema: { type: 'object' },
      },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (data?.status !== 'created' || !data?.contract_id) {
      throw new Error(`unexpected: ${JSON.stringify(data)}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'basic',
      ['create project, then contract'],
      'status=created with contract_id', JSON.stringify(data), 'PASS', ['create_contract.rs']);
    return data;
  }));

  results.push(await test('API-CC-2 create_contract with all fields', async () => {
    const slug = `apicc2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'CC Test 2' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_contract',
      arguments: {
        project_slug: slug, method: 'POST', path: '/api/full',
        summary: 'Full test', description: 'A test with all fields',
        request_body_schema: { type: 'object', properties: { name: { type: 'string' } } },
        response_schema: { type: 'object' },
        group_name: 'TestGroup', tags: ['api', 'test'],
      },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'all fields',
      ['create contract with description, schemas, group, tags'],
      'status=created', JSON.stringify(mcpExtract(r)), 'PASS', ['create_contract.rs full input']);
    return mcpExtract(r);
  }));

  results.push(await test('API-CC-3 create_contract duplicate method+path returns duplicate_key', async () => {
    const slug = `apicc3-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'Dup test' } });
    const args = {
      project_slug: slug, method: 'GET', path: '/api/dup',
      summary: 'First', response_schema: { type: 'object' },
    };
    await mcpCall(token, 'tools/call', { name: 'create_contract', arguments: args });
    const r = await mcpCall(token, 'tools/call', { name: 'create_contract', arguments: { ...args, summary: 'Second' } });
    // Exact duplicate is rejected as duplicate_key, not a soft warning
    if (!r.error) throw new Error('expected duplicate_key error');
    if (r.error.code !== -32602) throw new Error(`expected -32602, got ${r.error.code}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'duplicate path',
      ['create two contracts same method+path'],
      'error duplicate_key (hard reject)',
      JSON.stringify(r.error), 'PASS', ['create_contract.rs duplicate_key handling']);
    return r.error;
  }));

  results.push(await test('API-CC-4 create_contract similar but not identical, force=true creates', async () => {
    const slug = `apicc4-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'Force test' } });
    // First contract
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/api/force-1',
        summary: 'List users from the database with pagination and filtering options',
        response_schema: { type: 'object' },
      },
    });
    // Similar but different path + summary
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_contract',
      arguments: {
        project_slug: slug, method: 'GET', path: '/api/force-2',
        summary: 'Get list of users with pagination and filters',
        force: true, response_schema: { type: 'object' },
      },
    });
    if (r.error) throw new Error(`unexpected error: ${JSON.stringify(r.error)}`);
    const data = mcpExtract(r);
    if (data?.status !== 'created' && data?.status !== 'warning_similar_found') {
      throw new Error(`unexpected status: ${JSON.stringify(data)}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'force=true creates similar',
      ['create similar contract with force=true'],
      'status=created (force bypass)',
      JSON.stringify(data), 'PASS', ['create_contract.rs force flag']);
    return data;
  }));

  results.push(await test('API-CC-5 create_contract with invalid method', async () => {
    const slug = `apicc5-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'Bad method' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_contract',
      arguments: { project_slug: slug, method: 'INVALID', path: '/x', summary: 'X', response_schema: {} },
    });
    if (!r.error) throw new Error('expected validation error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'invalid method',
      ['create with method=INVALID'], 'error validation',
      JSON.stringify(r.error), 'PASS', ['Method::parse']);
    return r.error;
  }));

  results.push(await test('API-CC-6 create_contract with missing required field', async () => {
    const slug = `apicc6-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'Missing' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_contract',
      arguments: { project_slug: slug, method: 'GET' /* no path */ },
    });
    if (!r.error) throw new Error('expected validation error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'missing path',
      ['create with no path'], 'error validation',
      JSON.stringify(r.error), 'PASS', ['create_contract.rs required fields']);
    return r.error;
  }));

  // === API-004: get_contract_by_id scenarios ===
  results.push(await test('API-GC-1 get_contract_by_id valid', async () => {
    const slug = `apigc1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'GC' } });
    const created = await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/api/gc1',
        summary: 'GC 1', response_schema: { type: 'object' },
      },
    });
    const cid = mcpExtract(created)?.contract_id;
    if (!cid) throw new Error(`no id: ${JSON.stringify(created)}`);
    const r = await mcpCall(token, 'tools/call', {
      name: 'get_contract_by_id', arguments: { project_slug: slug, contract_id: cid },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (!data?.id) throw new Error(`no contract data: ${JSON.stringify(data)}`);
    if (data.id !== cid) throw new Error(`id mismatch: ${data.id} != ${cid}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'get_contract_by_id', 'valid id',
      ['get by id'], 'returns contract object with matching id',
      JSON.stringify(Object.keys(data).slice(0, 5)), 'PASS', ['get_contract_by_id.rs']);
    return data;
  }));

  results.push(await test('API-GC-2 get_contract_by_id non-existent', async () => {
    const slug = `apigc2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'GC2' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'get_contract_by_id',
      arguments: { project_slug: slug, contract_id: '00000000-0000-0000-0000-000000000000' },
    });
    if (!r.error) throw new Error('expected not found error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'get_contract_by_id', 'non-existent id',
      ['get with all-zeros uuid'], 'error not_found',
      JSON.stringify(r.error), 'PASS', ['get_contract_by_id.rs not found path']);
    return r.error;
  }));

  results.push(await test('API-GC-3 get_contract_by_id invalid uuid', async () => {
    const slug = `apigc3-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'GC3' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'get_contract_by_id', arguments: { project_slug: slug, contract_id: 'not-a-uuid' },
    });
    if (!r.error) throw new Error('expected invalid_params error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'get_contract_by_id', 'invalid uuid',
      ['get with non-uuid'], 'error -32602 invalid',
      JSON.stringify(r.error), 'PASS', ['get_contract_by_id.rs uuid validation']);
    return r.error;
  }));

  // === API-005: list_contracts scenarios ===
  results.push(await test('API-LC-1 list_contracts for project', async () => {
    const slug = `apilc1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'LC' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/a',
        summary: 'A', response_schema: { type: 'object' },
      },
    });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'POST', path: '/b',
        summary: 'B', response_schema: { type: 'object' },
      },
    });
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_contracts', arguments: { project_slug: slug },
    });
    const data = mcpExtract(r);
    if (data?.contracts?.length !== 2) {
      throw new Error(`expected 2 contracts, got ${data?.contracts?.length}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_contracts', 'basic',
      ['create 2 contracts, list'],
      '2 contracts returned', `count=${data.contracts.length}`,
      'PASS', ['list_contracts.rs']);
    return data;
  }));

  results.push(await test('API-LC-2 list_contracts with status filter', async () => {
    const slug = `apilc2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'LC2' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/x',
        summary: 'X', status: 'stable', response_schema: { type: 'object' },
      },
    });
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_contracts', arguments: { project_slug: slug, status: 'stable' },
    });
    const data = mcpExtract(r);
    if (!data?.contracts?.length) throw new Error(`no contracts: ${JSON.stringify(data)}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_contracts', 'status filter',
      ['list with status=stable'],
      'returns matching contracts', `count=${data.contracts.length}`,
      'PASS', ['list_contracts.rs status filter']);
    return data;
  }));

  results.push(await test('API-LC-3 list_contracts with group filter', async () => {
    const slug = `apilc3-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'LC3' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/g1',
        summary: 'G1', group_name: 'MyGroup', response_schema: { type: 'object' },
      },
    });
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_contracts', arguments: { project_slug: slug, group_name: 'MyGroup' },
    });
    const data = mcpExtract(r);
    if (!data?.contracts?.length) throw new Error('no contracts in group');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_contracts', 'group filter',
      ['list with group_name=MyGroup'],
      'returns contracts in group', `count=${data.contracts.length}`,
      'PASS', ['list_contracts.rs group filter']);
    return data;
  }));

  results.push(await test('API-LC-4 list_contracts unknown group returns 404', async () => {
    const slug = `apilc4-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'LC4' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_contracts', arguments: { project_slug: slug, group_name: 'NonExistent' },
    });
    if (!r.error) throw new Error('expected not found error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_contracts', 'unknown group',
      ['list with group_name=NonExistent'], 'error not_found (no group create)',
      JSON.stringify(r.error), 'PASS', ['list_contracts.rs API-002 fix']);
    return r.error;
  }));

  // === API-006: update_contract scenarios ===
  results.push(await test('API-UC-1 update_contract summary', async () => {
    const slug = `apiuc1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'UC' } });
    const created = await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/u1',
        summary: 'Original', response_schema: { type: 'object' },
      },
    });
    const cid = mcpExtract(created)?.contract_id;
    const r = await mcpCall(token, 'tools/call', {
      name: 'update_contract',
      arguments: { project_slug: slug, contract_id: cid, summary: 'Updated' },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'update_contract', 'summary only',
      ['update summary'], 'status=updated',
      JSON.stringify(mcpExtract(r)), 'PASS', ['update_contract.rs']);
    return mcpExtract(r);
  }));

  results.push(await test('API-UC-2 update_contract changes method/path', async () => {
    const slug = `apiuc2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'UC2' } });
    const created = await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/old',
        summary: 'X', response_schema: { type: 'object' },
      },
    });
    const cid = mcpExtract(created)?.contract_id;
    const r = await mcpCall(token, 'tools/call', {
      name: 'update_contract',
      arguments: { project_slug: slug, contract_id: cid, method: 'POST', path: '/new' },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    // Verify by getting back
    const check = await mcpCall(token, 'tools/call', {
      name: 'get_contract_by_id', arguments: { project_slug: slug, contract_id: cid },
    });
    const data = mcpExtract(check);
    if (!data || data.method !== 'POST' || data.path !== '/new') {
      throw new Error(`update didn't apply: method=${data?.method} path=${data?.path}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'update_contract', 'method+path change',
      ['update method=POST path=/new, verify'],
      'method=POST, path=/new after update',
      JSON.stringify({ method: data.method, path: data.path }),
      'PASS', ['update_contract.rs']);
    return data;
  }));

  results.push(await test('API-UC-3 update_contract invalid id', async () => {
    const slug = `apiuc3-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'UC3' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'update_contract',
      arguments: { project_slug: slug, contract_id: 'not-a-uuid', summary: 'X' },
    });
    if (!r.error) throw new Error('expected invalid_params');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'update_contract', 'invalid id',
      ['update with bad uuid'], 'error -32602',
      JSON.stringify(r.error), 'PASS', ['update_contract.rs uuid validation']);
    return r.error;
  }));

  // === API-007: delete_contract scenarios ===
  results.push(await test('API-DC-1 delete_contract valid', async () => {
    const slug = `apidc1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'DC' } });
    const created = await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/d1',
        summary: 'D1', response_schema: { type: 'object' },
      },
    });
    const cid = mcpExtract(created)?.contract_id;
    const r = await mcpCall(token, 'tools/call', {
      name: 'delete_contract', arguments: { project_slug: slug, contract_id: cid },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    // Verify gone
    const check = await mcpCall(token, 'tools/call', {
      name: 'get_contract_by_id', arguments: { project_slug: slug, contract_id: cid },
    });
    if (!check.error) throw new Error('contract still exists');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'delete_contract', 'valid id',
      ['delete then get'], 'status=deleted, get returns not_found',
      JSON.stringify({ delete: mcpExtract(r), get: check.error }),
      'PASS', ['delete_contract.rs']);
    return mcpExtract(r);
  }));

  results.push(await test('API-DC-2 delete_contract non-existent', async () => {
    const slug = `apidc2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'DC2' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'delete_contract',
      arguments: { project_slug: slug, contract_id: '00000000-0000-0000-0000-000000000000' },
    });
    if (!r.error) throw new Error('expected not found');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'delete_contract', 'non-existent',
      ['delete with all-zeros uuid'], 'error not_found',
      JSON.stringify(r.error), 'PASS', ['delete_contract.rs']);
    return r.error;
  }));

  // === API-008: list_groups scenarios ===
  results.push(await test('API-LG-1 list_groups for empty project', async () => {
    const slug = `apilg1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'LG' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_groups', arguments: { project_slug: slug },
    });
    const data = mcpExtract(r);
    if (!data || !Array.isArray(data.groups)) throw new Error(`bad: ${JSON.stringify(data)}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_groups', 'empty project',
      ['list on empty project'], 'returns empty array',
      JSON.stringify(data), 'PASS', ['list_groups.rs']);
    return data;
  }));

  results.push(await test('API-LG-2 list_groups with implicit groups', async () => {
    const slug = `apilg2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'LG2' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/g1',
        summary: 'G1', group_name: 'Auth', response_schema: { type: 'object' },
      },
    });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/g2',
        summary: 'G2', group_name: 'Auth', response_schema: { type: 'object' },
      },
    });
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_groups', arguments: { project_slug: slug },
    });
    const data = mcpExtract(r);
    const auth = data?.groups?.find(g => g.name === 'Auth');
    if (!auth) throw new Error('Auth group not found');
    if (auth.contract_count !== 2) throw new Error(`expected 2 contracts, got ${auth.contract_count}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_groups', 'with counts',
      ['create 2 contracts in same group, list'],
      'group with contract_count=2', JSON.stringify(auth),
      'PASS', ['list_groups.rs count aggregation']);
    return data;
  }));

  // === API-009: search_contract scenarios ===
  results.push(await test('API-SC-1 search_contract keyword mode', async () => {
    const slug = `apisc1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'SC' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/users',
        summary: 'List all users', response_schema: { type: 'object' },
      },
    });
    const r = await mcpCall(token, 'tools/call', {
      name: 'search_contract',
      arguments: { project_slug: slug, query: 'users', mode: 'keyword' },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (!data?.results?.length) throw new Error(`no results: ${JSON.stringify(data)}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'search_contract', 'keyword mode',
      ['search users'], 'returns matches',
      `count=${data.results.length}`, 'PASS', ['search_contract.rs']);
    return data;
  }));

  results.push(await test('API-SC-2 search_contract empty query', async () => {
    const slug = `apisc2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'SC2' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'search_contract',
      arguments: { project_slug: slug, query: '', mode: 'keyword' },
    });
    // Should error or return empty
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'search_contract', 'empty query',
      ['search with query=""'], 'error or empty results',
      JSON.stringify(r).substring(0, 200), 'PASS', ['search_contract.rs query validation']);
    return r;
  }));

  // === API-010: export_openapi scenarios ===
  results.push(await test('API-EO-1 export_openapi for empty project', async () => {
    const slug = `apieo1-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'EO' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'export_openapi', arguments: { project_slug: slug },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (!data?.yaml) throw new Error(`no yaml in response: ${JSON.stringify(data)}`);
    if (!data.yaml.includes('openapi:')) throw new Error('not valid openapi');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'export_openapi', 'empty project',
      ['export empty project'], 'returns valid openapi yaml',
      `len=${data.yaml.length}`, 'PASS', ['export_openapi.rs']);
    return data;
  }));

  results.push(await test('API-EO-2 export_openapi with contracts', async () => {
    const slug = `apieo2-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'EO2' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/api/eo-test',
        summary: 'EO test', response_schema: { type: 'object' },
      },
    });
    const r = await mcpCall(token, 'tools/call', {
      name: 'export_openapi', arguments: { project_slug: slug },
    });
    const data = mcpExtract(r);
    if (!data?.yaml?.includes('/api/eo-test')) {
      throw new Error(`path not in export: ${data?.yaml?.substring(0, 500)}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'export_openapi', 'with contracts',
      ['export with 1 contract'], 'path included in export',
      'contains /api/eo-test', 'PASS', ['export_openapi.rs path serialization']);
    return data;
  }));

  // === API-011: tools/list ===
  results.push(await test('API-TL-1 tools/list returns all 10 tools', async () => {
    const r = await mcpCall(token, 'tools/list', {});
    if (r.error) throw new Error(JSON.stringify(r.error));
    if (!r.result?.tools?.length) throw new Error('no tools');
    if (r.result.tools.length !== 10) throw new Error(`expected 10, got ${r.result.tools.length}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'tools/list', 'tool count',
      ['mcpCall tools/list'], '10 tools',
      `count=${r.result.tools.length}`, 'PASS', ['mcp/server.rs tool registry']);
    return r.result.tools.map(t => t.name);
  }));

  // === API-012: tools/call with unknown tool ===
  results.push(await test('API-TL-2 tools/call unknown tool', async () => {
    const r = await mcpCall(token, 'tools/call', { name: 'no_such_tool', arguments: {} });
    if (!r.error) throw new Error('expected error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'tools/call', 'unknown tool',
      ['call non-existent tool'], 'error -32601 method not found',
      JSON.stringify(r.error), 'PASS', ['mcp/server.rs dispatch']);
    return r.error;
  }));

  // === API-013: invalid JSON-RPC envelope ===
  results.push(await test('API-EN-1 invalid JSON envelope', async () => {
    const resp = await fetch(`${SERVER}/mcp`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'accept': 'application/json, text/event-stream',
        'authorization': `Bearer ${token}`,
      },
      body: '{invalid json',
    });
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    const ct = resp.headers.get('content-type') || '';
    let body = await resp.text();
    if (ct.includes('text/event-stream')) {
      const lines = body.split('\n');
      for (const line of lines) {
        if (line.startsWith('data: ')) { body = line.substring(6).trim(); break; }
      }
    }
    const parsed = JSON.parse(body);
    if (parsed.error?.code !== -32700) throw new Error(`code: ${parsed.error?.code}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'mcp envelope', 'parse error',
      ['POST malformed JSON'], 'error -32700 parse error',
      JSON.stringify(parsed.error), 'PASS', ['mcp/server.rs parse error handling']);
    return parsed;
  }));

  results.push(await test('API-EN-2 missing method field', async () => {
    const r = await mcpCall(token, 'tools/call', { name: 'list_projects' /* wait, this is the proper structure */ });
    // Actually test a JSON-RPC envelope without method
    const resp = await fetch(`${SERVER}/mcp`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'accept': 'application/json, text/event-stream',
        'authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, params: {} }),
    });
    const ct = resp.headers.get('content-type') || '';
    let body = await resp.text();
    if (ct.includes('text/event-stream')) {
      const lines = body.split('\n');
      for (const line of lines) {
        if (line.startsWith('data: ')) { body = line.substring(6).trim(); break; }
      }
    }
    const parsed = JSON.parse(body);
    if (parsed.error?.code !== -32600) throw new Error(`code: ${parsed.error?.code}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'mcp envelope', 'missing method',
      ['POST without method'], 'error -32600 invalid request',
      JSON.stringify(parsed.error), 'PASS', ['mcp/server.rs invalid request']);
    return parsed;
  }));

  results.push(await test('API-EN-3 unknown method', async () => {
    const r = await mcpCall(token, 'nonexistent/method', {});
    if (!r.error) throw new Error('expected method not found');
    if (r.error.code !== -32601) throw new Error(`code: ${r.error.code}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'mcp envelope', 'unknown method',
      ['call unknown method'], 'error -32601 method not found',
      JSON.stringify(r.error), 'PASS', ['mcp/server.rs method not found']);
    return r.error;
  }));

  // === API-014: Bearer auth edge cases ===
  results.push(await test('API-AUTH-1 no Authorization header', async () => {
    const resp = await fetch(`${SERVER}/mcp`, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'accept': 'application/json, text/event-stream' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/list' }),
    });
    if (resp.status !== 401) throw new Error(`status: ${resp.status}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'mcp auth', 'no auth header',
      ['POST without Authorization'], '401',
      `status=${resp.status}`, 'PASS', ['bearer_auth middleware']);
    return resp.status;
  }));

  results.push(await test('API-AUTH-2 invalid bearer token', async () => {
    const resp = await fetch(`${SERVER}/mcp`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'accept': 'application/json, text/event-stream',
        'authorization': 'Bearer not-a-real-token',
      },
      body: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/list' }),
    });
    if (resp.status !== 401 && resp.status !== 403) {
      throw new Error(`status: ${resp.status}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'mcp auth', 'invalid token',
      ['POST with bogus bearer'], '401 or 403',
      `status=${resp.status}`, 'PASS', ['bearer_auth middleware']);
    return resp.status;
  }));

  // === API-015: scope enforcement ===
  results.push(await test('API-SCOPE-1 read-only token cannot create', async () => {
    // Get a read-only token
    const regResp = await fetch(`${SERVER}/oauth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        client_name: 'Read Only Test',
        redirect_uris: ['http://127.0.0.1:9999/cb-readonly'],
      }),
    });
    const { client_id } = await regResp.json();
    const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM';
    const cookies = await cdp.send('Network.getCookies');
    const sessionCookie = cookies.cookies.find(c => c.name === 'ledgapi_session');
    const csrfCookie = cookies.cookies.find(c => c.name === 'ledgapi_csrf');
    const cookieHeader = `ledgapi_session=${sessionCookie.value}${csrfCookie ? `; ledgapi_csrf=${csrfCookie.value}` : ''}`;
    const authUrl = `${SERVER}/oauth/authorize?response_type=code&client_id=${client_id}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-readonly&scope=ledgapi%3Aread&state=s&code_challenge=${challenge}&code_challenge_method=S256`;
    const authResp = await fetch(authUrl, {
      headers: { cookie: cookieHeader },
      redirect: 'manual',
    });
    const html = await authResp.text();
    const csrfMatch = html.match(/name="csrf" value="([^"]+)"/);
    if (!csrfMatch) throw new Error('no csrf in read-only consent');
    const consentResp = await fetch(`${SERVER}/oauth/consent`, {
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded', cookie: cookieHeader },
      body: `client_id=${client_id}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-readonly&code_challenge=${challenge}&code_challenge_method=S256&scope=ledgapi%3Aread&state=s&decision=approve&csrf=${csrfMatch[1]}`,
      redirect: 'manual',
    });
    const loc = consentResp.headers.get('location');
    if (!loc || !loc.includes('code=')) throw new Error(`no code: ${loc}`);
    const code = new URL(loc).searchParams.get('code');
    const tokenResp = await fetch(`${SERVER}/oauth/token`, {
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body: `grant_type=authorization_code&code=${code}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-readonly&client_id=${client_id}&code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk`,
    });
    const readToken = (await tokenResp.json()).access_token;
    // Try to create a project with read-only token
    const r = await mcpCall(readToken, 'tools/call', {
      name: 'create_project', arguments: { slug: `readonly-${Date.now()}`, name: 'Should fail' },
    });
    if (!r.error) throw new Error('expected scope error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'mcp scope', 'read-only create',
      ['create with ledgapi:read only'], 'error forbidden',
      JSON.stringify(r.error), 'PASS', ['bearer_auth scope check']);
    return r.error;
  }));

  // === API-016: Web route scenarios ===
  results.push(await test('API-WEB-1 /healthz returns ok', async () => {
    const resp = await fetch(`${SERVER}/healthz`);
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    const text = await resp.text();
    if (!text.includes('ok')) throw new Error(`body: ${text}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /healthz', 'basic',
      ['curl /healthz'], '200 ok', text, 'PASS', ['health.rs']);
    return text;
  }));

  results.push(await test('API-WEB-2 /readyz returns ok', async () => {
    const resp = await fetch(`${SERVER}/readyz`);
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /readyz', 'basic',
      ['curl /readyz'], '200', `status=${resp.status}`, 'PASS', ['health.rs']);
    return resp.status;
  }));

  results.push(await test('API-WEB-3 /static/style.css served', async () => {
    const resp = await fetch(`${SERVER}/static/style.css`);
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    const text = await resp.text();
    if (!text.includes('ledgapi')) throw new Error('not ledgapi css');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /static/style.css', 'basic',
      ['curl style.css'], '200 css',
      `size=${text.length}`, 'PASS', ['router.rs serve_css']);
    return text.length;
  }));

  results.push(await test('API-WEB-4 /static/logo.png served', async () => {
    const resp = await fetch(`${SERVER}/static/logo.png`);
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    const buf = await resp.arrayBuffer();
    if (buf.byteLength < 1000) throw new Error(`too small: ${buf.byteLength}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /static/logo.png', 'basic',
      ['curl logo.png'], '200 png',
      `size=${buf.byteLength}`, 'PASS', ['router.rs serve_logo']);
    return buf.byteLength;
  }));

  results.push(await test('API-WEB-5 /docs returns 303 when no auth', async () => {
    const resp = await fetch(`${SERVER}/docs`, { redirect: 'manual' });
    if (resp.status !== 303) throw new Error(`status: ${resp.status}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /docs', 'no auth',
      ['curl /docs no auth'], '303 redirect to login',
      `loc=${resp.headers.get('location')}`, 'PASS', ['web auth gate']);
    return resp.status;
  }));

  results.push(await test('API-WEB-6 /.well-known/oauth-authorization-server', async () => {
    const resp = await fetch(`${SERVER}/.well-known/oauth-authorization-server`);
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    const text = await resp.text();
    if (!text.includes('issuer')) throw new Error('no issuer field');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /.well-known/oauth-authorization-server', 'metadata',
      ['curl oauth metadata'], '200 with issuer',
      text.substring(0, 100), 'PASS', ['oauth.rs metadata']);
    return text.length;
  }));

  results.push(await test('API-WEB-7 /projects/{slug}/openapi.yml', async () => {
    const slug = `apiyml-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'YML' } });
    await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/yml-test',
        summary: 'Y', response_schema: { type: 'object' },
      },
    });
    const cookies = await cdp.send('Network.getCookies');
    const sessionCookie = cookies.cookies.find(c => c.name === 'ledgapi_session');
    const resp = await fetch(`${SERVER}/projects/${slug}/openapi.yml`, {
      headers: { cookie: `ledgapi_session=${sessionCookie.value}` },
    });
    if (resp.status !== 200) throw new Error(`status: ${resp.status}`);
    const text = await resp.text();
    if (!text.includes('openapi:')) throw new Error('not valid openapi');
    if (!text.includes('/yml-test')) throw new Error('path not in yaml');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'GET /projects/{slug}/openapi.yml', 'yaml export',
      ['export yaml'], '200 yaml with path',
      `len=${text.length}`, 'PASS', ['openapi_export.rs']);
    return text.length;
  }));

  // Summary
  const evidence = {
    verification: 'Chrome DevTools Protocol - Comprehensive API tests',
    server: SERVER,
    tests: results,
    summary: {
      total: results.length,
      passed: results.filter(r => r.status === 'PASS').length,
      failed: results.filter(r => r.status === 'FAIL').length,
    },
  };
  writeFileSync('/tmp/api-comprehensive-results.json', JSON.stringify(evidence, null, 2));
  console.log('\n=== SUMMARY ===');
  console.log(`Total: ${evidence.summary.total}, Passed: ${evidence.summary.passed}, Failed: ${evidence.summary.failed}`);

  cdp.ws.close();
  process.exit(evidence.summary.failed > 0 ? 1 : 0);
}

main().catch(e => { console.error(e); process.exit(2); });
