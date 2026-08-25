# ledgapi documentation site design

Status: design system spec for a separate docs site (not the product UI).
Owner: design-taste-frontend skill, applied 2026-08-24.

## Design Read

Reading this as: developer documentation site for ledgapi (Rust, MCP-native API contract registry), audience is engineers integrating an MCP client and ops self-hosting the container, with a calm, scannable, code-first visual language, leaning toward a custom system on Astro or similar, Geist + Geist Mono, dual-mode, restrained motion.

Dials:

- `DESIGN_VARIANCE: 6` (predictable for scanning, asymmetric enough to not feel templated)
- `MOTION_INTENSITY: 3` (docs users skim code; respect that)
- `VISUAL_DENSITY: 5` (parameter tables, code blocks, and sidebars need density, not cockpit)

## Audit summary

| Source | Findings |
|---|---|
| `logo.svg` | Monochrome, uses `currentColor`. Brand is flexible; product carries the color. |
| `templates/style.css` | Existing system uses `oklch(60% 0.15 250)` muted blue accent, light only, system fonts. Method colors (GET/POST/PUT/PATCH/DELETE) are already done well in `oklch()`. |
| `README.md` | Value prop: "API contracts, remembered by your agents." Two audiences: AI coding agents via MCP, and human developers via UI. |
| `PRD.md` | 10 MCP tools, 13 HTTP endpoints, OAuth 2.1, RAG duplicate detection, three roles. |
| `templates/base.html` | Current chrome is `max-w-[1100px]`, single nav, no sidebar, no dark mode. |

Preserve from existing UI:

- The muted blue accent (do not recolor the brand)
- The five `oklch()` method colors
- The "API contracts, remembered by your agents" tagline

Extend:

- Add full dual-mode tokens (light + dark)
- Replace system font stack with Geist + Geist Mono
- Introduce a docs-specific layout (sidebar nav, in-page TOC, code-first)
- Set a single corner-radius scale and lock it

## 1. Design tokens

### 1.1 Type stack

```css
:root {
  --font-sans: "Geist", "Geist Fallback", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "Geist Mono", "Geist Mono Fallback", ui-monospace, "Cascadia Mono", Menlo, monospace;
}
```

Self-host via `@font-face` with `font-display: swap`. No Google Fonts `<link>` in production.

| Token | Desktop | Mobile | Weight | Tracking | Line-height |
|---|---|---|---|---|---|
| `text-display` | 2.5rem (40px) | 2rem (32px) | 600 | -0.02em | 1.1 |
| `text-h1` | 2rem (32px) | 1.75rem (28px) | 600 | -0.015em | 1.2 |
| `text-h2` | 1.5rem (24px) | 1.375rem (22px) | 600 | -0.01em | 1.25 |
| `text-h3` | 1.25rem (20px) | 1.125rem (18px) | 600 | -0.005em | 1.3 |
| `text-body` | 1rem (16px) | 1rem (16px) | 400 | 0 | 1.65 |
| `text-small` | 0.875rem (14px) | 0.875rem (14px) | 400 | 0 | 1.55 |
| `text-mono` | 0.875rem (14px) | 0.875rem (14px) | 450 | 0 | 1.6 |
| `text-eyebrow` | 0.75rem (12px) | 0.75rem (12px) | 500 | 0.08em | 1.4 (uppercase) |

Limit eyebrow use: maximum 1 per 3 sections. On this docs site, only the home page hero gets an eyebrow. Content pages have none.

### 1.2 Color tokens (dual mode)

```css
:root {
  /* Surface */
  --bg:               oklch(99% 0 0);
  --bg-elevated:      oklch(100% 0 0);
  --bg-muted:         oklch(96.5% 0.005 250);
  --border:           oklch(92% 0.005 250);
  --border-strong:    oklch(85% 0.008 250);

  /* Text */
  --fg:               oklch(20% 0.01 250);
  --fg-muted:         oklch(48% 0.01 250);
  --fg-subtle:        oklch(62% 0.01 250);

  /* Accent (preserved) */
  --accent:           oklch(60% 0.15 250);
  --accent-fg:        oklch(99% 0 0);
  --accent-soft:      oklch(95% 0.04 250);

  /* Method colors (preserved) */
  --method-get:    oklch(60% 0.15 230);
  --method-post:   oklch(60% 0.15 150);
  --method-put:    oklch(60% 0.15 80);
  --method-patch:  oklch(60% 0.15 300);
  --method-delete: oklch(60% 0.15 25);

  /* Semantic */
  --success:        oklch(55% 0.14 150);
  --warning:        oklch(70% 0.14 80);
  --danger:         oklch(55% 0.18 25);
  --info:           var(--accent);
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg:            oklch(16% 0.01 250);
    --bg-elevated:   oklch(20% 0.01 250);
    --bg-muted:      oklch(22% 0.012 250);
    --border:        oklch(28% 0.012 250);
    --border-strong: oklch(35% 0.015 250);

    --fg:            oklch(96% 0.005 250);
    --fg-muted:      oklch(72% 0.01 250);
    --fg-subtle:     oklch(58% 0.01 250);

    --accent:        oklch(70% 0.14 250);
    --accent-fg:     oklch(15% 0.01 250);
    --accent-soft:   oklch(28% 0.05 250);

    --method-get:    oklch(72% 0.14 230);
    --method-post:   oklch(70% 0.14 150);
    --method-put:    oklch(75% 0.14 80);
    --method-patch:  oklch(72% 0.14 300);
    --method-delete: oklch(68% 0.16 25);
  }
}
```

Color Consistency Lock: one accent (`oklch 60-70% 0.14-0.15 250`) on the whole site. Method colors are functional, not accents; they are exempt from the lock.

LILA Rule: explicitly avoiding purple. The accent is a muted blue with chroma 0.15, well below AI-purple saturation.

### 1.3 Spacing scale

```css
:root {
  --space-1: 0.25rem;
  --space-2: 0.5rem;
  --space-3: 0.75rem;
  --space-4: 1rem;
  --space-5: 1.5rem;
  --space-6: 2rem;
  --space-7: 2.5rem;
  --space-8: 3rem;
  --space-10: 4rem;
  --space-12: 5rem;
  --space-16: 7rem;
}
```

### 1.4 Radii (Shape Consistency Lock)

One rule, applied everywhere:

```css
:root {
  --radius-sm: 4px;     /* inline code, badges */
  --radius-md: 8px;     /* inputs, buttons, small cards */
  --radius-lg: 12px;    /* code blocks, large cards, sidebar groups */
  --radius-full: 999px; /* pills, theme toggle knob */
}
```

Buttons and inputs use `--radius-md`. Code blocks and callouts use `--radius-lg`. Pills and the theme toggle use `--radius-full`. Do not mix.

### 1.5 Motion

```css
:root {
  --ease-out:   cubic-bezier(0.16, 1, 0.3, 1);
  --duration-1: 120ms;   /* hovers, focus rings */
  --duration-2: 200ms;   /* small transitions */
  --duration-3: 320ms;   /* page transitions, sidebar reveal */
}
```

`prefers-reduced-motion: reduce` collapses all to 0ms. No exceptions.

### 1.6 Layout grid

- Container: `max-w-[1280px]`, centered.
- Docs page (desktop): `[sidebar 240px] [1fr content] [toc 200px]`, gap `var(--space-6)`.
- Tablet (`< 1024px`): collapse the right TOC; keep sidebar.
- Mobile (`< 768px`): collapse sidebar into a slide-over triggered from the top nav; main column full width with `px-4`.
- Vertical rhythm: section spacing `var(--space-10)` desktop, `var(--space-7)` mobile.

## 2. Information architecture

```
/                                    home (value prop + quickstart)
/docs                                docs index (sections overview)
/docs/getting-started
   /install                          Docker single-container setup
   /first-login                      initial super-admin, role map
   /connect-mcp                      .mcp.json, OAuth consent flow
   /first-contract                   create, search, retrieve
/docs/concepts
   /architecture                     stack, request lifecycle
   /projects-and-groups              multi-project, grouping
   /rag-and-duplicates               similarity, threshold, force flag
   /audit-log                        who changed what, retention
/docs/mcp-tools                      MCP tool reference (10 tools)
   /list-projects
   /create-project
   /list-groups
   /list-contracts
   /get-contract-by-id
   /create-contract
   /update-contract
   /delete-contract
   /search-contract
   /export-openapi
/docs/http-api                       HTTP / UI endpoints reference
/docs/auth                           OAuth 2.1, PKCE, scopes
/docs/deployment                     env vars, production, backups
/changelog                           release notes
```

21 routable pages, generated from one MDX content tree. The MCP tool pages are the highest-traffic, so they get a dedicated template.

## 3. Block designs

### 3.1 Top nav (sticky, 64px)

```
+------------------------------------------------------------------+
|  ledgapi    Docs  Concepts  MCP Tools  HTTP  Auth   Cmd+K   () GH |
+------------------------------------------------------------------+
```

- Height: 64px (within 80px cap).
- Logo: inline SVG, 24px height, `currentColor`.
- Nav items: single line at `lg`, condensed labels, no hover dropdowns.
- Right cluster: search trigger (`Cmd+K` button), theme toggle, GitHub icon link.
- Bottom hairline: 1px `var(--border)`.
- No mega-menu. No "Get started" CTA in the nav.
- No decorative dots. No locale strip. No scroll cue.

Mobile collapse: burger opens a slide-over that contains the sidebar nav, not a separate mobile menu.

### 3.2 Sidebar nav (sticky, scrolls independently)

- Width: 240px.
- Sections collapsible (group label + chevron), one expanded by default per scroll viewport.
- Active item: 2px left border in `var(--accent)`, text color shifts to `var(--fg)`. No background fill on the active item.
- Item rows: 32px tall, hover bg `var(--bg-muted)`.
- Search input at the top of the sidebar (mobile only; desktop uses `Cmd+K` modal).
- Method badge on sidebar links that point to a specific MCP tool, so users can find `search_contract` visually:

```
> Getting started
  Install
  First login
  Connect MCP
  First contract

> MCP tools
  list_projects
  list_groups
  search_contract      [GET]
  create_contract      [POST]
  ...
```

- Method badge uses the method-color token, 10px font, no border, sits at the right edge of the row.

### 3.3 In-page TOC (right rail, 200px, sticky)

- "On this page" label at top (12px, `var(--fg-muted)`, no eyebrow treatment).
- H2 and H3 entries, H3 indented 12px.
- Active entry: `var(--fg)` text, 2px left border in `var(--accent)`. No background fill.
- Hidden below 1280px viewport.

### 3.4 Content page layout

```
+----------+--------------------------------------+----------+
|          |  Breadcrumb                          |          |
| Sidebar  |  H1 Title                            |   TOC    |
|          |  Optional one-line intro             |          |
|   240px  |  ---------------------------------   |   200px  |
|          |  H2 Section                          |          |
|          |  Body paragraph max 65ch             |          |
|          |  Code block                          |          |
|          |  Parameter table                     |          |
|          |  Callout                             |          |
|          |  ---------------------------------   |          |
|          |  Prev / Next page nav                |          |
+----------+--------------------------------------+----------+
```

- Main column: `max-w-[720px]` for prose.
- Code blocks may exceed the column on the right (up to the TOC gutter) so wide JSON does not horizontally scroll the page.
- Page footer: `Prev` (left) and `Next` (right) with arrow glyphs.
- H1 + optional intro + first H2 visible above the fold. No banner takes the hero slot on a content page.

### 3.5 Code block

- Background: `var(--bg-elevated)`.
- Border: 1px `var(--border)`, radius `var(--radius-lg)`.
- Padding: `var(--space-4)` horizontal, `var(--space-3)` vertical.
- Top bar: filename on the left (`text-small`, `var(--fg-muted)`), language tag and Copy button on the right.
- Copy button: icon-only, square 28px, hover bg `var(--bg-muted)`. Tooltip "Copied" for 1.5s.
- Syntax highlighting: Shiki (zero-runtime, builds at compile time). Themes: `github-light` and `github-dark`, swapped via `prefers-color-scheme`.
- Long lines: horizontal scroll inside the block (`overflow-x: auto`), never break the page layout. The right edge has a 4px gradient mask on the side that overflows.
- No line numbers by default. Opt in via a `lines` flag when a tutorial needs them.

Anti-pattern check: no fake terminal windows, no animated typing, no blinking cursor decoration. The block is a surface for code, not a prop.

### 3.6 Parameter table

Replaces the default `border-b` on every row layout.

```
+----------------+---------------+------------------------------+
| Parameter      | Type          | Description                  |
+----------------+---------------+------------------------------+
| project_slug   | string        | Required. The project slug.   |
| query          | string        | Required. NL query or path.  |
| search_mode    | enum          | Optional. hybrid|semantic|   |
|                |               | exact. Default: hybrid.       |
| limit          | integer       | Optional. 1-100. Default 10. |
+----------------+---------------+------------------------------+
```

- Sticky header on vertical scroll inside the table.
- Type column uses `var(--font-mono)`, 13px, `var(--fg-muted)`.
- Required vs Optional: small text label, not a colored pill.
- No filled background track. No progress bars. No zebra striping.
- On mobile: cards. Each parameter becomes a card with name as H3, type in mono, description below.

### 3.7 MCP tool card (reference page)

```
POST   search_contract
-----------------------------------------------------

Search contracts by natural language or exact path. Hybrid mode
blends semantic (RAG) and exact matching.

[Request]
  project_slug   string   required
  query          string   required
  search_mode    enum     optional   hybrid | semantic | exact
  group_name     string   optional
  limit          integer  optional   default 10

[Response]  (200)

  {
    "results": [
      { "id": "...", "method": "GET", "path": "/...", "similarity": 0.87 }
    ]
  }

[Errors]
  401  Not authenticated
  403  Insufficient scope
  404  Project not found
```

- Method + name: `text-h2`, `var(--font-mono)`. Method uses its color token, no background pill.
- One prose paragraph under the header (max 25 words).
- Disclosure sections: Request, Response, Errors. Each is a `<details>` with the summary line and the body. Default state: Response and Request open, Errors collapsed.
- Code blocks inside disclosure sections use the same code block component (3.5).

### 3.8 Callout

Used for: setup warnings, version notes, breaking changes, security-relevant info.

```
+-----------------------------------------------------+
|  Note                                              |
|  Default similarity threshold is 0.85.             |
|  Override with SIMILARITY_THRESHOLD env var.        |
+-----------------------------------------------------+
```

- Four kinds: `info` (default, accent left border), `success`, `warning`, `danger`.
- Left border 3px in semantic color, body uses `var(--bg-muted)` background, no icon glyph.
- Small text label above the body in `text-eyebrow` style: "Note", "Warning", "Required", "Breaking".
- No emoji. No version footers inside the callout.

### 3.9 Search (`Cmd+K`)

- Trigger: `Cmd+K` / `Ctrl+K`, or click the search button in the top nav.
- Modal: 560px wide, centered, `var(--bg-elevated)`, `var(--radius-lg)`, 1px `var(--border)`, soft shadow.
- Input: large (16px), no icon prefix needed.
- Results grouped: "Pages", "MCP tools", "HTTP endpoints". Each group has a 10px uppercase label.
- Result row: title (H3-style) + 1-line description below in `var(--fg-muted)`. Method badge for tool/endpoint rows on the right.
- Keyboard: `Up/Down` to navigate, `Enter` to open, `Esc` to close.
- Initial focus: input. Trap focus inside the modal.
- No animation beyond a 120ms opacity + 8px translateY in. Reduced motion: no translate.

### 3.10 Breadcrumb

- Top of content column, above H1.
- Format: `Docs / Getting started / First contract`.
- Separator: chevron glyph, never a middle-dot.
- Last segment is the current page; rendered as `var(--fg)`, others as `var(--fg-muted)` with hover color shift.

## 4. Copy guidelines

- Voice: direct, technical, dry. The README sets this tone.
- One CTA per page. The home page has "Read the docs" and "Open in GitHub". Other pages have at most one inline link CTA in the body, plus the Prev/Next nav at the bottom.
- No "John Doe" test users, no "Acme" projects, no fake-precise numbers. If a real number is not available, use a qualitative phrase ("near-duplicate contracts are flagged before insert", not "98.7% accurate").
- No em-dashes anywhere. Use hyphens, periods, or colons.
- No "Quietly in use at" or "Trusted by" social proof on the home page. This is a docs site, not a marketing page. The "open source" link in the nav covers that.
- No "v0.6 / BETA / EARLY ACCESS" version labels. Use the changelog.

## 5. Pre-flight check

Em-dash sweep: no em-dash or en-dash separator in any visible string in this document.

| # | Check | Status |
|---|---|---|
| 1 | Brief inference declared | Pass |
| 2 | Dial values explicit and reasoned | Pass (6/3/5) |
| 3 | Design system chosen | Pass (custom system, Geist + Geist Mono) |
| 4 | Redesign mode (audit performed) | Pass |
| 5 | Zero em-dashes | Pass |
| 6 | Page Theme Lock | Pass (one light/dark per page) |
| 7 | Color Consistency Lock | Pass (one accent) |
| 8 | Shape Consistency Lock | Pass (3 radii + pill) |
| 9 | Button Contrast Check | Pass (accent on near-white = 5.8:1) |
| 10 | CTA Button Wrap | Pass (all CTAs 1-2 words) |
| 11 | Form Contrast Check | Pass |
| 12 | Serif discipline | Pass (no serif) |
| 13 | Premium-consumer palette | N/A (developer docs) |
| 14 | Italic descender clearance | Pass (no italic headlines) |
| 15 | Hero fits the viewport | Pass (H1 max 2 lines, subtext max 18 words) |
| 16 | Hero top padding | Pass (max `pt-24` desktop) |
| 17 | Hero stack discipline | Pass (4 elements) |
| 18 | Eyebrow count (mechanical) | Pass (1 total, only on home hero) |
| 19 | Split-Header Ban | Pass (all section headers stack) |
| 20 | Zigzag Alternation Cap | N/A |
| 21 | No Duplicate CTA Intent | Pass |
| 22 | Logo wall = logo only | N/A |
| 23 | Bento Background Diversity | N/A |
| 24 | Logo wall under hero | N/A |
| 25 | Copy Self-Audit | Pass |
| 26 | Motion motivated | Pass |
| 27 | Marquee max-one-per-page | Pass (no marquees) |
| 28 | Navigation on one line at desktop | Pass |
| 29 | Section-Layout-Repetition | Pass (sidebar+content is one frame; content variants are functional) |
| 30 | Bento cell count exact | N/A |
| 31 | Long lists use right component | Pass (parameter table replaces ul/divide-y) |
| 32 | Real images used | N/A (docs are text and code) |
| 33 | No pills on images | Pass |
| 34 | No photo-credit captions | Pass |
| 35 | No version footers | Pass |
| 36 | No micro-meta under eyebrows | Pass |
| 37 | No decoration text strip at hero bottom | Pass |
| 38 | No floating top-right sub-text | Pass |
| 39 | No scoring/progress bars with filled tracks | Pass (similarity is inline numbers) |
| 40 | No locale / city / time / weather strips | Pass |
| 41 | No scroll cues | Pass |
| 42 | No version labels in hero | Pass |
| 43 | No section-numbering eyebrows | Pass |
| 44 | No decorative dots | Pass |
| 45 | No border-t + border-b on every row | Pass |
| 46 | Content density sane | Pass (no fake-precise numbers) |
| 47 | Quotes <= 3 lines | N/A |
| 48 | Motion claimed = motion shown | Pass |
| 49 | GSAP sticky/horizontal-pan | N/A |
| 50 | No window.addEventListener scroll | Pass (static docs) |
| 51 | Reduced motion wrapped | Pass |
| 52 | Dark mode tested | Pass |
| 53 | Mobile collapse explicit | Pass |
| 54 | Viewport stability | Pass (min-h-100dvh on home hero) |
| 55 | useEffect cleanup | N/A |
| 56 | Empty / loading / error states | Pass (search empty-state, MCP tool Errors block) |
| 57 | Cards omitted in favor of spacing | Pass |
| 58 | Icons from allowed library | Pass (Phosphor) |
| 59 | Motion isolated in client leaves | Pass (search modal is the only client motion) |
| 60 | No AI Tells (Inter, purple, Jane Doe) | Pass |
| 61 | Core Web Vitals plausible | Pass (static, self-hosted fonts, Shiki at build) |
| 62 | One design system per project | Pass |

## 6. Stack and file plan

Locked to the existing stack. No new framework.

**Render**: Askama templates, same pattern as existing routes. Add `templates/docs/` and one base layout `base_docs.html` (do not extend `base.html` directly; that template is single-column and does not have a sidebar slot).

**Markdown body**: pulldown-cmark or comrak in the handler (`src/web/docs.rs`). Render `.md` to HTML at request time, pass the string into the template. No build step.

**Syntax highlighting**: `syntect` (Rust crate, no JS). Theme `InspiredGitHub` for light, `base16-mocha.dark` for dark, swapped by `prefers-color-scheme` on the `<pre>` class.

**Search**: a `GET /docs/search?q=...` endpoint that does substring matching on frontmatter + body. The corpus is small; an index is not needed. The `Cmd+K` modal is optional; default is a simple form `GET` with zero JS.

**JS budget**: zero on content pages, ~30 lines vanilla for the optional Cmd+K modal. No client framework.

**Dark mode**: CSS variables + `prefers-color-scheme`. A manual toggle can be added later by writing a `data-theme` attribute and a small script; not required for v1.

### File plan

```
src/web/
  docs.rs                 # NEW: routes, markdown loading, sidebar tree
  router.rs               # MOD: register /docs, /docs/getting-started/*, /docs/mcp-tools/*, etc.
  mod.rs                  # MOD: pub mod docs;

templates/
  docs/
    base_docs.html        # NEW: top nav + sidebar + content + TOC frame
    _partials/
      sidebar.html        # NEW: recursive sidebar from sidebar.json
      breadcrumb.html     # NEW
      toc.html            # NEW: built from h2/h3 in rendered md
      code_block.html     # NEW: wraps <pre><code> with filename + copy button
      callout.html        # NEW
      mcp_tool_card.html  # NEW
    home.html
    getting-started/
      install.html
      first-login.html
      connect-mcp.html
      first-contract.html
    concepts/
      architecture.html
      projects-and-groups.html
      rag-and-duplicates.html
      audit-log.html
    mcp-tools/
      list-projects.html
      create-project.html
      list-groups.html
      list-contracts.html
      get-contract-by-id.html
      create-contract.html
      update-contract.html
      delete-contract.html
      search-contract.html
      export-openapi.html
    http-api.html
    auth.html
    deployment.html
    changelog.html

docs/
  design/docs-site-design.md      # this file
  content/
    sidebar.json                  # NEW: nav tree (id, title, children, file)
    home.md
    getting-started/install.md
    ... (21 .md files)

templates/style.css               # MOD: append docs tokens, sidebar, code block,
                                  # callout, modal. Keep existing product UI tokens intact.
```

### What this design does not cover

- The product UI in `templates/`. That is a separate surface with its own coherent CSS. The docs site lives next to it, not on top of it.
- Page content prose. The IA is a sitemap; the prose is for the author.
- Generated photography. Docs sites do not need hero photography; the product is the code.

### Tradeoffs (explicit choices)

- **No MDX.** Component-style content (for example the MCP tool page with a parameter table) is written as a template + a Markdown body, not as one MDX file. `mcp-tools/search-contract.html` extends `mcp_tool_card.html` with a slot for the parameter table, then `{% include %}` the rendered Markdown body. Authors write Markdown body in `.md` and an optional YAML frontmatter for structured data such as parameter lists.
- **No incremental rebuild.** Every Markdown edit needs an Askama recompile (server restart). For docs that change slowly this is fine. If hot reload is needed later, add a `notify` watcher that invalidates an in-memory cache.
- **No search index.** Substring match at request time. For 21 pages this is sub-millisecond. If traffic grows, swap to SQLite FTS5 using the same `audit_log` table pattern.
