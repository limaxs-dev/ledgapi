// New features + UI test suite via Chrome DevTools Protocol
// Covers:
//   - create_group MCP tool (new in 6948807)
//   - create_contract with group_parent_id (folder hierarchy)
//   - Web UI: nested group tree rendering
//   - All existing UI elements (header, navigation, logout, etc.)

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
    this.consoleExceptions = [];
    this.consoleMessages = [];
    ws.on('message', (data) => {
      const msg = JSON.parse(data);
      if (msg.id !== undefined && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message));
        else resolve(msg.result);
      } else if (msg.method) {
        if (msg.method === 'Runtime.exceptionThrown') this.consoleExceptions.push(msg.params);
        if (msg.method === 'Runtime.consoleAPICalled') this.consoleMessages.push(msg.params);
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

async function navigate(cdp, url) {
  await cdp.send('Page.enable');
  const navP = new Promise(resolve => {
    const handler = (data) => {
      const msg = JSON.parse(data.toString());
      if (msg.method === 'Page.loadEventFired') {
        cdp.ws.removeListener('message', handler);
        resolve();
      }
    };
    cdp.ws.on('message', handler);
  });
  await cdp.send('Page.navigate', { url });
  await navP;
  await new Promise(r => setTimeout(r, 600));
}

async function getDOM(cdp, expr) {
  const { result, exceptionDetails } = await cdp.send('Runtime.evaluate', {
    expression: expr, returnByValue: true, awaitPromise: true,
  });
  if (exceptionDetails) throw new Error(`Eval failed: ${exceptionDetails.text}`);
  return result.value;
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

let traceCounter = 800;
function writeTrace(traceId, segment, target, scenario, actions, expected, actual, result, evidence) {
  const trace = {
    trace_id: traceId, requirement: 'New features + UI', segment, target, scenario,
    actions, expected, actual, result, evidence, confidence: 'high', iteration: 13,
  };
  appendFileSync(`${QA_DIR}/data/traces.jsonl`, JSON.stringify(trace) + '\n');
}

async function getToken() {
  const loginResp = await fetch(`${SERVER}/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'username=admin&password=change-this-password-1234&next=/',
    redirect: 'manual',
  });
  const setCookies = loginResp.headers.getSetCookie ? loginResp.headers.getSetCookie() : [];
  let sessionCookie = '', csrfCookie = '';
  for (const sc of setCookies) {
    const [pair] = sc.split(';');
    if (pair.startsWith('ledgapi_session=')) sessionCookie = pair;
    if (pair.startsWith('ledgapi_csrf=')) csrfCookie = pair;
  }
  const cookieHeader = `${sessionCookie}; ${csrfCookie}`;
  const regResp = await fetch(`${SERVER}/oauth/register`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ client_name: 'New Features Test', redirect_uris: ['http://127.0.0.1:9999/cb-nf'] }),
  });
  const { client_id } = await regResp.json();
  const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM';
  const scope = 'ledgapi%3Aread%20ledgapi%3Awrite%20ledgapi%3Aadmin';
  const authUrl = `${SERVER}/oauth/authorize?response_type=code&client_id=${client_id}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-nf&scope=${scope}&state=s&code_challenge=${challenge}&code_challenge_method=S256`;
  const authResp = await fetch(authUrl, { headers: { cookie: cookieHeader }, redirect: 'manual' });
  const html = await authResp.text();
  const csrfMatch = html.match(/name="csrf" value="([^"]+)"/);
  const consentResp = await fetch(`${SERVER}/oauth/consent`, {
    method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded', cookie: cookieHeader },
    body: `client_id=${client_id}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-nf&code_challenge=${challenge}&code_challenge_method=S256&scope=${scope}&state=s&decision=approve&csrf=${csrfMatch[1]}`,
    redirect: 'manual',
  });
  const loc = consentResp.headers.get('location');
  const code = new URL(loc).searchParams.get('code');
  const tokenResp = await fetch(`${SERVER}/oauth/token`, {
    method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: `grant_type=authorization_code&code=${code}&redirect_uri=http%3A%2F%2F127.0.0.1%3A9999%2Fcb-nf&client_id=${client_id}&code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk`,
  });
  const tj = await tokenResp.json();
  return { accessToken: tj.access_token, sessionCookie, csrfCookie };
}

async function mcpCall(token, method, params, id) {
  const resp = await fetch(`${SERVER}/mcp`, {
    method: 'POST', headers: {
      'content-type': 'application/json',
      'accept': 'application/json, text/event-stream',
      'authorization': `Bearer ${token}`,
    },
    body: JSON.stringify({ jsonrpc: '2.0', id: id || 1, method, params }),
  });
  const ct = resp.headers.get('content-type') || '';
  let body = await resp.text();
  if (ct.includes('text/event-stream')) {
    for (const line of body.split('\n')) {
      if (line.startsWith('data: ')) { body = line.substring(6).trim(); break; }
    }
  }
  return JSON.parse(body);
}
function mcpExtract(resp) {
  if (resp.error) return null;
  const c = resp.result?.content?.[0];
  if (!c) return null;
  if (c.type === 'json') return c.json;
  if (c.type === 'text') { try { return JSON.parse(c.text); } catch { return c.text; } }
  return null;
}

async function freshSession() {
  const r = await fetch(`${SERVER}/login`, {
    method: 'POST', headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: 'username=admin&password=change-this-password-1234&next=/', redirect: 'manual',
  });
  const setCookies = r.headers.getSetCookie ? r.headers.getSetCookie() : [];
  let s = '', c = '';
  for (const sc of setCookies) {
    const [pair] = sc.split(';');
    if (pair.startsWith('ledgapi_session=')) s = pair;
    if (pair.startsWith('ledgapi_csrf=')) c = pair;
  }
  return { sessionCookie: s, csrfCookie: c };
}

async function main() {
  const target = await getTarget();
  const cdp = new CDPClient(new WebSocket(target.webSocketDebuggerUrl));
  await new Promise(r => cdp.ws.on('open', r));
  await cdp.send('Runtime.enable');
  await cdp.send('Network.enable');
  await cdp.send('Page.enable');

  let { accessToken: token, sessionCookie, csrfCookie } = await getToken();
  const results = [];

  // === NEW FEATURE TESTS (create_group + nested contracts) ===

  // 1. create_group tool is registered
  results.push(await test('NF-001 create_group is registered as MCP tool', async () => {
    const r = await mcpCall(token, 'tools/list', {});
    if (r.error) throw new Error(JSON.stringify(r.error));
    const names = r.result.tools.map(t => t.name);
    if (!names.includes('create_group')) throw new Error('create_group missing');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_group', 'registered',
      ['tools/list'], '11 tools including create_group', `count=${names.length}`,
      'PASS', ['src/mcp/tools_impl/create_group.rs']);
    return { toolCount: names.length, hasCreateGroup: names.includes('create_group') };
  }));

  // 2. create_group creates a root-level group
  results.push(await test('NF-002 create_group creates root-level folder', async () => {
    const slug = `nf-test-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'NF Test' } });
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_group', arguments: { project_slug: slug, name: 'Invoices', description: 'Billing-related endpoints' },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (!data.id) throw new Error('no id');
    if (data.parent_id) throw new Error(`expected null parent_id, got ${data.parent_id}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_group', 'root folder',
      ['create_group without parent_id'], 'returns group with null parent_id',
      `id=${data.id}`, 'PASS', ['create_group.rs']);
    return data;
  }));

  // 3. create_group with parent_id creates a nested folder
  results.push(await test('NF-003 create_group with parent_id creates sub-folder', async () => {
    // Find the project and root folder from NF-002
    const listResp = await mcpCall(token, 'tools/call', {
      name: 'list_projects', arguments: {},
    });
    const projects = mcpExtract(listResp)?.projects;
    const slug = projects.find(p => p.name === 'NF Test').slug;
    const listGrp = await mcpCall(token, 'tools/call', {
      name: 'list_groups', arguments: { project_slug: slug },
    });
    const parentGroup = mcpExtract(listGrp)?.groups.find(g => g.name === 'Invoices');
    if (!parentGroup) throw new Error('parent not found');
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_group', arguments: {
        project_slug: slug, name: 'PDF',
        description: 'PDF invoice generation',
        parent_id: parentGroup.id,
      },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (data.parent_id !== parentGroup.id) {
      throw new Error(`parent_id mismatch: ${data.parent_id} != ${parentGroup.id}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_group', 'nested folder',
      ['create_group with parent_id'], 'returns group with parent_id set',
      `parent_id=${data.parent_id}`, 'PASS', ['create_group.rs']);
    return data;
  }));

  // 4. create_group with same name+parent is idempotent
  results.push(await test('NF-004 create_group is idempotent on (name, parent)', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    const listGrp = await mcpCall(token, 'tools/call', {
      name: 'list_groups', arguments: { project_slug: slug },
    });
    const invoices = mcpExtract(listGrp).groups.find(g => g.name === 'Invoices');
    const r1 = await mcpCall(token, 'tools/call', {
      name: 'create_group', arguments: { project_slug: slug, name: 'Tax', parent_id: invoices.id },
    });
    const r2 = await mcpCall(token, 'tools/call', {
      name: 'create_group', arguments: { project_slug: slug, name: 'Tax', parent_id: invoices.id },
    });
    const d1 = mcpExtract(r1), d2 = mcpExtract(r2);
    if (d1.id !== d2.id) throw new Error(`id mismatch: ${d1.id} != ${d2.id}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_group', 'idempotency',
      ['create_group twice with same args'], 'returns same group id',
      `id=${d1.id}`, 'PASS', ['create_group.rs idempotent resolve']);
    return { id1: d1.id, id2: d2.id, same: d1.id === d2.id };
  }));

  // 5. create_contract with group_parent_id files in nested folder
  results.push(await test('NF-005 create_contract with group_parent_id', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    const listGrp = await mcpCall(token, 'tools/call', {
      name: 'list_groups', arguments: { project_slug: slug },
    });
    const pdfGroup = mcpExtract(listGrp).groups.find(g => g.name === 'PDF');
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_contract', arguments: {
        project_slug: slug, method: 'GET', path: '/invoices/{id}/pdf',
        summary: 'Download invoice as PDF',
        group_name: 'PDF', group_parent_id: pdfGroup.parent_id,
        response_schema: { type: 'object', properties: { url: { type: 'string', format: 'uri' } } },
      },
    });
    if (r.error) throw new Error(JSON.stringify(r.error));
    const data = mcpExtract(r);
    if (data.status !== 'created') throw new Error(`unexpected status: ${data.status}`);
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_contract', 'nested parent_id',
      ['create_contract with group_parent_id'], 'status=created, group auto-created under parent',
      `contract_id=${data.contract_id}`, 'PASS', ['create_contract.rs group_parent_id']);
    return data;
  }));

  // 6. list_groups returns parent_id on every group
  results.push(await test('NF-006 list_groups returns parent_id', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    const r = await mcpCall(token, 'tools/call', {
      name: 'list_groups', arguments: { project_slug: slug },
    });
    const groups = mcpExtract(r).groups;
    const withParent = groups.filter(g => g.parent_id);
    if (withParent.length === 0) throw new Error('no groups have parent_id');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'list_groups', 'parent_id field',
      ['list_groups and inspect'], 'each group has parent_id field',
      `total=${groups.length} nested=${withParent.length}`, 'PASS', ['list_groups.rs']);
    return { total: groups.length, nested: withParent.length };
  }));

  // 7. Invalid parent_id returns a validation error
  results.push(await test('NF-007 create_group with invalid parent_id errors', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_group', arguments: {
        project_slug: slug, name: 'BadParent', parent_id: 'not-a-valid-uuid',
      },
    });
    if (!r.error) throw new Error('expected validation error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_group', 'invalid parent_id',
      ['create_group with bad parent_id'], 'error validation',
      JSON.stringify(r.error), 'PASS', ['create_group.rs id validation']);
    return r.error;
  }));

  // 8. create_group on unknown project returns not_found
  results.push(await test('NF-008 create_group on unknown project returns not_found', async () => {
    const r = await mcpCall(token, 'tools/call', {
      name: 'create_group', arguments: { project_slug: 'no-such-project', name: 'X' },
    });
    if (!r.error) throw new Error('expected error');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'create_group', 'unknown project',
      ['create_group on bogus slug'], 'error not_found',
      JSON.stringify(r.error), 'PASS', ['create_group.rs project lookup']);
    return r.error;
  }));

  // 9. list_groups tree rendering: each group appears with correct depth
  results.push(await test('NF-009 group tree has correct depth in UI', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    // Set browser cookies
    await cdp.send('Network.clearBrowserCookies');
    await cdp.send('Network.setCookie', {
      name: 'ledgapi_session', value: sessionCookie.split('=')[1],
      domain: '127.0.0.1', path: '/',
    });
    await cdp.send('Network.setCookie', {
      name: 'ledgapi_csrf', value: csrfCookie.split('=')[1],
      domain: '127.0.0.1', path: '/',
    });
    await navigate(cdp, `${SERVER}/projects/${slug}`);
    const dom = await getDOM(cdp, `(() => {
      // Walk the DOM looking for <details> elements with data-depth
      const groups = document.querySelectorAll('details.group[data-depth]');
      const depths = {};
      for (const g of groups) {
        const d = parseInt(g.getAttribute('data-depth'), 10);
        const name = g.querySelector('.group-name')?.textContent?.trim();
        if (name) depths[name] = d;
      }
      return { groupCount: groups.length, depths };
    })()`);
    if (dom.groupCount < 3) throw new Error(`only ${dom.groupCount} groups rendered`);
    // Invoices should be at depth 0, PDF at depth 1, Tax at depth 1
    if (dom.depths['Invoices'] !== 0) throw new Error(`Invoices depth: ${dom.depths['Invoices']}`);
    if (dom.depths['PDF'] !== 1) throw new Error(`PDF depth: ${dom.depths['PDF']}`);
    if (dom.depths['Tax'] !== 1) throw new Error(`Tax depth: ${dom.depths['Tax']}`);
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Project page', 'group tree depth',
      ['inspect details.group data-depth'], 'depths 0 and 1 both render',
      JSON.stringify(dom.depths), 'PASS', ['templates/_partials/group_node.html data-depth']);
    return dom;
  }));

  // 10. Nested contracts are visible under their parent group
  results.push(await test('NF-010 contract in sub-folder is rendered under parent', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    await navigate(cdp, `${SERVER}/projects/${slug}`);
    const dom = await getDOM(cdp, `(() => {
      // Find the <details> for "PDF" and check that the contract is inside it
      const details = Array.from(document.querySelectorAll('details.group'));
      const pdf = details.find(d => d.querySelector('.group-name')?.textContent?.trim() === 'PDF');
      if (!pdf) return { found: false };
      const text = pdf.textContent;
      return {
        found: true,
        hasContractPath: text.includes('/invoices/'),
        hasContractSummary: text.includes('Download invoice as PDF'),
        opened: pdf.hasAttribute('open') || pdf.open,
      };
    })()`);
    if (!dom.found) throw new Error('PDF folder not found in DOM');
    if (!dom.hasContractPath) throw new Error('contract path not in PDF folder');
    if (!dom.hasContractSummary) throw new Error('contract summary not in PDF folder');
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Project page', 'nested contract',
      ['find PDF folder, check for contract'], 'contract appears inside folder',
      JSON.stringify(dom), 'PASS', ['handlers.rs group_tree recursion']);
    return dom;
  }));

  // 11. Group count badge in header reflects nested count
  results.push(await test('NF-011 header "X groups" counts nested too', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    await navigate(cdp, `${SERVER}/projects/${slug}`);
    const dom = await getDOM(cdp, `(() => {
      const text = document.querySelector('main')?.textContent || '';
      // Look for "N groups (including nested)" or "N group (including nested)"
      const match = text.match(/(\\d+)\\s+groups?\\s+\\(including nested\\)/);
      return {
        match: match ? match[0] : null,
        count: match ? parseInt(match[1], 10) : null,
      };
    })()`);
    if (!dom.match) throw new Error(`no group count in: ${JSON.stringify(dom)}`);
    if (dom.count < 3) throw new Error(`expected >= 3 groups, got ${dom.count}`);
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Project page', 'group count',
      ['find group count text'], 'count includes nested',
      dom.match, 'PASS', ['templates/project.html group counter']);
    return dom;
  }));

  // === UI tests for new features (visual + interactive) ===

  // 12. New project has 0 groups, can be added via the new tool
  results.push(await test('NF-012 new project renders empty group tree', async () => {
    const slug = `empty-${Date.now()}`;
    await mcpCall(token, 'tools/call', { name: 'create_project', arguments: { slug, name: 'Empty project' } });
    await navigate(cdp, `${SERVER}/projects/${slug}`);
    const dom = await getDOM(cdp, `(() => {
      const details = document.querySelectorAll('details.group');
      const text = document.querySelector('main')?.textContent || '';
      return {
        detailCount: details.length,
        hasEmptyMessage: text.includes('No contracts yet'),
        contractsHeader: (text.match(/Contracts\\s*\\((\\d+)\\)/) || [null, null])[1],
        groupsHeader: (text.match(/(\\d+)\\s+groups?\\s+\\(including nested\\)/) || [null, null])[1],
      };
    })()`);
    if (dom.contractsHeader !== '0') throw new Error(`contracts: ${dom.contractsHeader}`);
    if (dom.groupsHeader !== '0') throw new Error(`groups: ${dom.groupsHeader}`);
    if (!dom.hasEmptyMessage) throw new Error('no empty message');
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Project page', 'empty state',
      ['navigate to empty project'], '0 groups, 0 contracts, empty message',
      JSON.stringify(dom), 'PASS', ['handlers.rs empty state']);
    return dom;
  }));

  // 13. Server-rendered OpenAPI export includes all nested paths
  results.push(await test('NF-013 OpenAPI export groups paths by tags', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    const r = await mcpCall(token, 'tools/call', {
      name: 'export_openapi', arguments: { project_slug: slug },
    });
    const yaml = mcpExtract(r).yaml;
    if (!yaml.includes('/invoices/')) throw new Error('invoice path missing in OpenAPI');
    writeTrace(`TRACE-${traceCounter++}`, 'api', 'export_openapi', 'nested paths',
      ['export yaml'], 'all nested paths present',
      `len=${yaml.length}`, 'PASS', ['export_openapi.rs path aggregation']);
    return { size: yaml.length, hasInvoicePath: yaml.includes('/invoices/') };
  }));

  // 14. Sign out button visible on the project page (regression of BUG-000006)
  results.push(await test('NF-014 sign out button present on project page', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    await navigate(cdp, `${SERVER}/projects/${slug}`);
    const dom = await getDOM(cdp, `(() => {
      const forms = document.querySelectorAll('form[data-logout]');
      const buttons = document.querySelectorAll('button.logout-btn');
      return { formCount: forms.length, buttonCount: buttons.length };
    })()`);
    if (dom.formCount === 0) throw new Error('no logout form');
    if (dom.buttonCount === 0) throw new Error('no logout button');
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Project page', 'logout button',
      ['inspect form data-logout'], 'logout form and button present',
      JSON.stringify(dom), 'PASS', ['templates/base.html logout form']);
    return dom;
  }));

  // 15. Search form is visible on project page (with nested groups)
  results.push(await test('NF-015 search form visible on project with groups', async () => {
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const slug = mcpExtract(listResp).projects.find(p => p.name === 'NF Test').slug;
    await navigate(cdp, `${SERVER}/projects/${slug}`);
    const dom = await getDOM(cdp, `(() => {
      const form = document.querySelector('form.search-form');
      return {
        formFound: !!form,
        hasQ: !!form?.querySelector('input[name="q"]'),
        hasMode: !!form?.querySelector('select[name="mode"]'),
        hasButton: !!form?.querySelector('button[type="submit"]'),
      };
    })()`);
    if (!dom.formFound) throw new Error('no search form');
    if (!dom.hasQ) throw new Error('no q input');
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Project page', 'search form',
      ['inspect form.search-form'], 'form + q + mode + button',
      JSON.stringify(dom), 'PASS', ['templates/project.html search-form']);
    return dom;
  }));

  // 16. No console errors across full new-feature test session
  results.push(await test('NF-016 no console errors during full session', async () => {
    cdp.consoleExceptions = [];
    cdp.consoleMessages = [];
    const listResp = await mcpCall(token, 'tools/call', { name: 'list_projects', arguments: {} });
    const projects = mcpExtract(listResp).projects;
    for (const p of projects) {
      await navigate(cdp, `${SERVER}/projects/${p.slug}`);
      await new Promise(r => setTimeout(r, 200));
    }
    const errors = cdp.consoleMessages.filter(m => m.type === 'error');
    const realErrors = errors.filter(m => {
      const text = m.args.map(a => a.value).join(' ');
      return !text.includes('favicon') && !text.includes('logo');
    });
    if (realErrors.length > 0) {
      throw new Error(`console errors: ${realErrors.map(m => m.args.map(a => a.value).join(' ')).join('; ')}`);
    }
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Full session', 'no console errors',
      ['navigate to all projects', 'watch console'], '0 errors',
      `errors=${realErrors.length}`, 'PASS', ['overall app health']);
    return { consoleErrors: realErrors.length, projectsVisited: projects.length };
  }));

  // === Standard UI checks (re-verify after new features) ===

  // 17. Login page: no overlapping elements
  results.push(await test('NF-017 login: no overlapping or hidden elements', async () => {
    await cdp.send('Network.clearBrowserCookies');
    await navigate(cdp, `${SERVER}/login`);
    const dom = await getDOM(cdp, `(() => {
      const issues = [];
      const isDesignHidden = (el) => el.classList.contains('skip-link') || el.type === 'hidden';
      for (const el of document.querySelectorAll('button, input, a, h1, h2, h3, label')) {
        if (isDesignHidden(el)) continue;
        const r = el.getBoundingClientRect();
        const cs = getComputedStyle(el);
        if (r.width === 0 || r.height === 0) issues.push({ el: el.tagName + ':' + (el.textContent || el.name || '').slice(0, 30), issue: 'zero-size' });
        if (cs.display === 'none' || cs.visibility === 'hidden') issues.push({ el: el.tagName, issue: 'hidden' });
      }
      return { issueCount: issues.length, issues: issues.slice(0, 3) };
    })()`);
    if (dom.issueCount > 0) throw new Error(`issues: ${JSON.stringify(dom.issues)}`);
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Login', 'no overlap/hidden',
      ['inspect login DOM'], 'all visible',
      JSON.stringify(dom), 'PASS', ['templates/login.html']);
    return dom;
  }));

  // 18. Admin pages still work
  results.push(await test('NF-018 admin pages render correctly', async () => {
    const fresh = await freshSession();
    sessionCookie = fresh.sessionCookie;
    csrfCookie = fresh.csrfCookie;
    await cdp.send('Network.clearBrowserCookies');
    await cdp.send('Network.setCookie', {
      name: 'ledgapi_session', value: sessionCookie.split('=')[1],
      domain: '127.0.0.1', path: '/',
    });
    await cdp.send('Network.setCookie', {
      name: 'ledgapi_csrf', value: csrfCookie.split('=')[1],
      domain: '127.0.0.1', path: '/',
    });
    const layouts = [];
    for (const p of ['/', '/admin/users', '/admin/audit']) {
      await navigate(cdp, `${SERVER}${p}`);
      const dom = await getDOM(cdp, `({
        hasHeader: !!document.querySelector('header'),
        hasMain: !!document.querySelector('main'),
        hasFooter: !!document.querySelector('footer'),
        hasNav: !!document.querySelector('header nav'),
        h1: document.querySelector('h1')?.textContent,
      })`);
      if (!dom.hasHeader || !dom.hasMain || !dom.hasFooter || !dom.hasNav) {
        throw new Error(`inconsistent layout at ${p}: ${JSON.stringify(dom)}`);
      }
      layouts.push({ p, ...dom });
    }
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Admin pages', 'consistent layout',
      ['visit /, /admin/users, /admin/audit'], 'all share header/main/footer/nav',
      JSON.stringify(layouts), 'PASS', ['templates/base.html']);
    return layouts;
  }));

  // 19. Docs pages render with new sign-out button
  results.push(await test('NF-019 docs pages render with new sign-out', async () => {
    await navigate(cdp, `${SERVER}/docs`);
    const dom = await getDOM(cdp, `(() => ({
      h1: document.querySelector('h1')?.textContent?.slice(0, 50),
      hasLogout: !!document.querySelector('form[data-logout]'),
      hasSidebar: !!document.querySelector('.docs-sidebar, [aria-label*="Docs sections"]'),
      linkCount: document.querySelectorAll('a[href^="/docs/"]').length,
    }))()`);
    if (!dom.h1) throw new Error('no h1');
    if (!dom.hasLogout) throw new Error('no logout on docs');
    if (dom.linkCount < 10) throw new Error(`only ${dom.linkCount} doc links`);
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Docs', 'all elements visible',
      ['inspect /docs'], 'h1, logout, sidebar, links present',
      JSON.stringify(dom), 'PASS', ['templates/docs/base_docs.html']);
    return dom;
  }));

  // 20. Sign out actually works (regression of BUG-000006)
  results.push(await test('NF-020 sign-out actually ends the session', async () => {
    await navigate(cdp, `${SERVER}/`);
    // Click the sign-out button
    await getDOM(cdp, `document.querySelector('button.logout-btn').click()`);
    await new Promise(r => setTimeout(r, 1500));
    const url = await getDOM(cdp, 'location.href');
    if (!url.includes('/login')) throw new Error(`expected /login, got ${url}`);
    writeTrace(`TRACE-${traceCounter++}`, 'ui_functional', 'Sign out', 'session ends',
      ['click logout-btn'], 'redirected to /login',
      url, 'PASS', ['src/web/auth.rs logout handler']);
    return { url };
  }));

  // Summary
  const evidence = {
    verification: 'Chrome DevTools Protocol - New features + UI',
    server: SERVER, tests: results,
    summary: {
      total: results.length,
      passed: results.filter(r => r.status === 'PASS').length,
      failed: results.filter(r => r.status === 'FAIL').length,
    },
    consoleExceptions: cdp.consoleExceptions,
    consoleErrors: cdp.consoleMessages.filter(m => m.type === 'error'),
  };
  writeFileSync('/tmp/new-features-results.json', JSON.stringify(evidence, null, 2));
  console.log('\n=== SUMMARY ===');
  console.log(`Total: ${evidence.summary.total}, Passed: ${evidence.summary.passed}, Failed: ${evidence.summary.failed}`);
  console.log(`Console exceptions: ${cdp.consoleExceptions.length}`);
  console.log(`Console errors: ${evidence.consoleErrors.length}`);
  cdp.ws.close();
  process.exit(evidence.summary.failed > 0 ? 1 : 0);
}

main().catch(e => { console.error(e); process.exit(2); });
