# Design System: ledgapi

## 1. Visual Theme & Atmosphere

A quiet, ledger-grade interface. The product is a contract registry for developers and AI agents; the UI is the audit surface, not the stage. Atmosphere: clinical precision with one charged accent that means business. Tables, code, and status badges carry the typographic weight; chrome recedes.

- **Density:** 4 — Daily App Balanced, leaning sparse. Information-dense where the data demands it (endpoint tables, audit log); airy on chrome surfaces.
- **Variance:** 4 — Offset Asymmetric, but earned. Split layouts and off-grid composition on hero and docs surfaces; internal app pages use generous centered content. Never artsy-chaotic.
- **Motion:** 2 — Static Restrained. State transitions only. No perpetual loops, no decorative reveals, no bounce easing. The interface disappears into the task.

This is the opposite of "premium creative agency." ledgapi's brand personality (`PRODUCT.md`) is precise, trustworthy, quiet — the surface should feel like opening a well-kept ledger, not a launch page.

## 2. Color Palette & Roles

### Light surface

- **Canvas Bone** (`#F7F7F5`) — Primary background surface. Warm-neutral off-white, not cold zinc.
- **Pure Surface** (`#FFFFFF`) — Cards, table containers, raised panels.
- **Ink** (`#1A1A1A`) — Primary text and logo foreground. Near-black with a hair of warmth; never `#000000`. Matches the logo-mark's monochrome foreground so brand tile and text feel like one ink.
- **Steel Mute** (`#52525B`) — Secondary text, descriptions, axis labels, metadata, table helper text. Zinc-600.
- **Whisper Border** (`rgba(228,228,231,0.85)`) — 1px structural lines, table dividers, card outlines. Zinc-200 with opacity so dividers feel like graphite, not chrome.
- **Surface Tint** (`rgba(212,255,58,0.06)`) — Whisper background for active rows, selected list items, focused form fields. A whisper of lime, never a flood.

### Dark surface

- **Canvas Ink** (`#0A0A0B`) — Primary background surface. Off-black, not crushed.
- **Raised Surface** (`#161618`) — Cards, panels, table containers. One step lighter than canvas.
- **Inverted Text** (`#F4F4F5`) — Primary text on dark surfaces. Zinc-100.
- **Mute on Dark** (`#A1A1AA`) — Secondary text. Zinc-400.
- **Whisper Border Dark** (`rgba(63,63,70,0.7)`) — Zinc-700 at 0.7 opacity.
- **Surface Tint Dark** (`rgba(212,255,58,0.10)`) — Active row tint, slightly stronger on dark to remain perceptible against the deeper surface.

### Accent — Electric Lime

- **Electric Lime** (`#D4FF3A`) — Single accent. Used for actions and state only. Logo brand-block tile fill, primary CTA fill, active nav indicator, focus ring, "active" / "settled" status badge. Ink-on-lime contrast ratio: 15.2:1 (WCAG AAA, well above AA floor).
- **Voltage Green** (`#B8E020`) — Hover and active states for lime fills. Deeper but still charged; reads as "pressed," not "different color."
- **Lime Whisper** (`rgba(212,255,58,0.06)` light / `0.10` dark) — Background tint only. Selected table row, focused input fill, hover surface for lime-stamped list items.

### Accent discipline (loud color, quiet hand)

The accent is one charged color; over-use is the single biggest design failure mode for this product. Hard rules:

1. **One accent per view, never per surface.** A page may use lime as fill on the CTA and as a 1px focus ring on a focused input, but never as page background, section divider, or body text.
2. **Never lime as a large background fill.** Lime may fill a button (~40px tall), a brand tile (~40–80px), a status badge (~20px), or a 1px stroke. Lime as a section or hero background is banned — it reads as a marketing splash, not a registry.
3. **Never lime on lime.** Lime badge on lime CTA, lime background under lime icon — both rejected. Ink goes on lime; nothing else.
4. **No lime gradients.** No lime-to-lime, lime-to-yellow, lime-to-cyan. Solid fills only. Gradients ending in lime are banned by `PRODUCT.md`'s "no drenched color" rule.
5. **No lime glow.** No `box-shadow: 0 0 20px rgba(212,255,58,...)`. No lime drop-shadows. No neon outer glow.
6. **Lime is the positive state.** Active, settled, current, valid. Absence of lime is the negative state. Destructive actions never use lime — they use Ink fill on light surfaces, raised-surface fill on dark. (Per `PRODUCT.md`: "restraint: one accent, used for actions and state only.")
7. **Lime body text is banned above 16px.** Lime as inline text fill on running copy reads as marketing copy, not data. Lime in badges, KPI tiles, and 1px strokes only.

## 3. Typography Rules

Type is the hero — tables, code, and badges carry this product. Three families, no exceptions.

- **Display / Headlines:** `Cabinet Grotesk` — Track-tight at `-0.02em`, weight-driven hierarchy (500 for H1, 600 for H2, 700 reserved for brand wordmark only). Headlines range `clamp(1.875rem, 4vw, 3rem)`. Never wrap to more than 4 lines — if a headline runs long, rewrite it.
- **Body:** `Satoshi` — Relaxed leading at `1.55`, base `15px` (desktop) and `14px` (mobile floor). Body width capped at `65ch`. Secondary copy in Steel Mute. Never use Satoshi for numbers in dense contexts — see Mono.
- **Mono:** `JetBrains Mono` — All numbers in tables (balances, IDs, hashes, timestamps, latencies, version strings), code blocks, JSON payloads, API paths, OAuth client IDs, audit log entries. Tabular figures (`font-variant-numeric: tabular-nums`) on every mono use. This is the typography that earns its keep in a ledger — give it the same care as the headlines.
- **Banned:** `Inter`, system defaults (`-apple-system`, `Segoe UI`, `Roboto`), generic serifs (`Times New Roman`, `Georgia`, `Garamond`). Serif is banned entirely in this product — there is no editorial context where it earns its place.

Headlines must never wrap to 6+ lines. If a hero headline breaks awkwardly, restructure the sentence — asymmetric width and `max-width: 24ch` on headline containers prevent wrap without compromise.

## 4. Component Stylings

### Buttons

- **Primary:** Lime fill (`#D4FF3A`), Ink text (`#1A1A1A`), no border, radius `8px`, height `40px` desktop / `44px` mobile floor. Padding `0 16px`. Weight 500. Tactile: `transform: translateY(1px)` on `:active`. No outer glow, no drop-shadow.
- **Secondary:** Transparent fill, `1px Ink` border at `0.15` opacity, Ink text. Hover: `rgba(26,26,26,0.04)` Whisper fill.
- **Tertiary / ghost:** Ink text, no border, no fill. Hover: Whisper fill at `0.04` opacity.
- **Destructive:** Ink fill on light surfaces, raised-surface fill on dark. Text in Bone (`#F7F7F5`) on light, Inverted Text on dark. **Never red.** Destructive = absence of lime, not presence of red.
- **Disabled:** `0.4` opacity, `cursor: not-allowed`, no hover transform.

### Tables (the data hero)

- **Container:** Pure Surface fill, `1px Whisper Border` outline, radius `12px`. No shadow on the container.
- **Header row:** Zinc-100 fill (`#F4F4F5`), Ink text, weight 500, uppercase tracking `0.04em`, `11px`. Sticky on scroll within containers.
- **Data rows:** `48px` min height (desktop), `44px` mobile. `1px` top divider between rows, no row borders inside cells. Hover: Lime Whisper fill. Selected: Lime Whisper fill + `2px` lime left edge.
- **Numerics:** JetBrains Mono, tabular figures, right-aligned. Currency formatting consistent across the app (`$1,234.56`, never `$1234.56`).
- **Status badges in rows:** Inline badge at end of row, not a separate column.
- **Empty state inside table:** "No contracts registered yet" + a single CTA. Never "No data." Never an illustration with rounded blobs (banned by `PRODUCT.md`).

### Status badges

Small, monospace, all-caps, `11px`. Four states — three colors plus absence.

- **Active / Settled / Valid:** Lime fill, Ink text. This is the only place lime fills a container that small.
- **Pending / Draft / In review:** Zinc-100 fill (`#F4F4F5` light / `#27272A` dark), Steel Mute text.
- **Stale / Deprecated / Failed:** Ink fill on light (`#1A1A1A` with Bone text), raised-surface fill on dark. Reads as "quietly serious."
- **Destructive / Removed:** same as Stale — never a red badge. Negative state is communicated by absence of lime, not presence of red.

Badge height `20px`, padding `0 8px`, radius `4px`. No icons inside badges — text only, monospace.

### Cards

Used sparingly. ledgapi is data-led; cards wrap a single concept, not a nested hierarchy.

- **Container:** Pure Surface, `1px Whisper Border`, radius `12px`, padding `24px`. Shadow `0 1px 2px rgba(26,26,26,0.04)` only — no elevated card shadow on a registry surface.
- **Use for:** endpoint summary, group summary, user card, audit event highlight. **Not for:** table rows (use row dividers), nav items (use borderless list), or nested components.
- **Never card-on-card-on-card.** A card containing a card containing a card is rejected. If you need that depth, restructure.

### Inputs

- **Label:** Above input, `13px`, Ink, weight 500.
- **Helper text:** Below input, `12px`, Steel Mute.
- **Input field:** `40px` height desktop / `44px` mobile, `1px Whisper Border`, radius `8px`, `8px 12px` padding. Background Pure Surface.
- **Focus state:** `2px` Electric Lime outer ring (use `:focus-visible` only), no default blue.
- **Error state:** Helper text in `#B45309` (amber). Never red. Never icon-only — always include the message.
- **Disabled:** `0.5` opacity, `cursor: not-allowed`.
- **Monospace inputs:** Use for IDs, hashes, paths, OAuth client secrets. JetBrains Mono.

### Nav (header)

- **Header:** `64px` height, Pure Surface fill, `1px` Whisper Border bottom. Sticky on scroll.
- **Brand wordmark:** logo-mark (`40px`) + "ledgapi" wordmark in Cabinet Grotesk 600, `18px`, Ink.
- **Nav links:** Satoshi `14px`, Steel Mute, hover Ink. Active link: Ink + `2px` Electric Lime bottom border (the only lime stroke in the nav).
- **No icon-spam sidebar.** Top-level nav is enough. If a section needs sub-nav, it lives in a left rail (`200px` wide, no icons, just text labels).
- **Mobile:** Horizontal scroll for top-level nav, never a hamburger drawer hiding the structure.

### Documentation surfaces

- **Two-column layout:** Left rail `240px` for TOC, right column fluid for content. Both use the same neutral surface.
- **Code blocks:** JetBrains Mono `14px`, `1px Whisper Border`, radius `8px`, padding `16px`, surface tint of `rgba(26,26,26,0.03)` (whisper-ink, not whisper-lime).
- **Inline code:** JetBrains Mono `13px`, Whisper Border, `2px 6px` padding, radius `4px`.
- **API endpoint badges:** `GET /v1/contracts/{id}` style. Method in JetBrains Mono, weight 700, color-coded by method:
  - `GET` → Steel Mute (read)
  - `POST` → Ink (write)
  - `PATCH` → Ink
  - `DELETE` → Ink fill on Bone (destructive)
  - Never lime on the method badge — lime is for status, not verbs.

### Empty states

- **Composition:** A single, restrained line illustration (ink stroke, 1.5px, no fill, no lime) OR an icon-only composition (24px Ink stroke icon). Never a rounded-blob illustration. Never an emoji.
- **Copy:** One-sentence explanation of why the surface is empty + one CTA. "No contracts registered yet. Register your first endpoint to start the audit log."
- **Layout:** Centered, `max-width: 360px`, generous vertical padding (`clamp(3rem, 6vw, 5rem)`).

### Loaders

- **Skeletal shimmer:** Matching the exact dimensions of the content it's replacing. Shimmer gradient: `rgba(228,228,231,0.4) → rgba(228,228,231,0.1) → rgba(228,228,231,0.4)`, 1.5s linear loop. Radius matches the component it's standing in for.
- **No circular spinners.** Spinners read as legacy finance apps.
- **Latency-budget UI:** When an operation is slow (>500ms), show a "Working…" text label beside the shimmer. When complete, fade shimmer out at `200ms` ease-out.

### Error states

- **Inline banner:** `1px Whisper Border` with `2px` left edge in `#B45309` (amber). Background Bone. Copy: "We couldn't reach the registry. Retry in 12s." Plain language, no stack trace in the UI surface.
- **Form errors:** Helper text below the field in `#B45309`.
- **Full-page errors (500, 404):** Centered, large headline + one-line explanation + one action. Quiet tone — "This page isn't in the registry." not "Oops! Something went wrong!"

## 5. Layout Principles

- **Grid:** 12-column CSS Grid, `24px` gutter, max-width `1200px` centered for app surfaces. `1400px` for docs surfaces (reading width benefits from more room).
- **Hero / marketing surfaces (docs landing, README):** Asymmetric split. 7-column headline + supporting copy, 5-column composition (a code block, a brand tile, or a quiet diagram — never a stock photo). Never centered. Headlines use `clamp(2rem, 5vw, 3.5rem)`.
- **Internal app surfaces (projects list, contracts table, audit log):** Generous centered content. Single column with `max-width: 960px`. Tables stretch to the available width — data benefits from room.
- **Section spacing:** `clamp(3rem, 6vw, 5rem)` vertical rhythm on docs surfaces. `clamp(1.5rem, 3vw, 2.5rem)` on app surfaces.
- **Internal padding:** `32px` desktop, `20px` mobile.
- **Full-height sections:** `min-h-[100dvh]`. Never `h-screen` (iOS Safari catastrophic jump).
- **No flexbox percentage math — CSS Grid only.**
- **No gradient backgrounds.** No lime-to-anything, no surface-to-surface. Solid fills only.
- **No overlapping elements.** Every element gets clean spatial ownership. No absolutely-positioned decorative shapes peeking from behind cards.

## 6. Motion & Interaction

The interface is restrained by mandate. Motion communicates state, never decoration.

- **Default transition:** `150ms cubic-bezier(0.2, 0, 0, 1)` — Material-style ease-out. Fast enough to feel responsive, slow enough to read.
- **Spring physics:** Not used by default. The skill default of `stiffness: 100, damping: 20` is rejected for this product — it reads as playful. Linear/cubic-bezier transitions for everything.
- **State transitions only:** Hover, focus, active, route change, modal open/close. Nothing else animates.
- **Staggered list reveals:** Optional, capped at 4 items, `40ms` cascade max. Beyond 4 items, no stagger — instant render reads as "this is a registry, not a feed."
- **Perpetual micro-loops:** Banned. No pulse, no typewriter, no floating dot, no shimmer on idle content. Shimmer is reserved for loaders.
- **Route changes:** `100ms` cross-fade on the content area only. No slide, no parallax.
- **Modal/drawer open:** `200ms` slide+fade. Drawer width `400px` desktop, full-width mobile.
- **Performance:** Animate only `transform` and `opacity`. No animating `width`, `height`, `top`, `left`. Use `will-change` sparingly — only on currently-animating elements.
- **Reduced motion (`prefers-reduced-motion: reduce`):** Disable all non-essential transitions. Keep `0.01ms` opacity fades for state changes only.

## 7. Anti-Patterns (Banned)

Combined from `PRODUCT.md` anti-references and the Stitch design-taste defaults.

### Brand-anchored bans (from `PRODUCT.md`)

- ❌ **No gradient heroes.** No surface-to-surface, no lime-to-anything. Solid fills only.
- ❌ **No drenched color.** Lime is an accent, not a backdrop. Never a hero background, never a section divider fill.
- ❌ **No icon-spam sidebars.** Text labels, no icons in nav unless they replace a verb.
- ❌ **No nested cards on cards on cards.** One level of card hierarchy max.
- ❌ **No bounce easing.** Linear / cubic-bezier only.
- ❌ **No decorative motion.** No perpetual loops, no parallax, no scroll-triggered reveals.
- ❌ **No rounded-blob illustration.** Ink-stroke diagrams, monoline icons, or nothing.
- ❌ **No playful startup aesthetics.** No hand-drawn underlines, no casual copy, no exclamation points.

### Stitch design-taste defaults

- ❌ **No emojis anywhere.**
- ❌ **No `Inter` font.** Cabinet Grotesk for display, Satoshi for body, JetBrains Mono for data.
- ❌ **No pure black (`#000000`).** Ink is `#1A1A1A` — a hair of warmth, matches the logo foreground.
- ❌ **No neon outer glows.** No lime drop-shadows, no `box-shadow: 0 0 Npx rgba(...)`.
- ❌ **No oversaturated accents.** Lime is the chosen single accent; saturation is its character, restraint in usage is the discipline.
- ❌ **No excessive gradient text on large headers.**
- ❌ **No custom mouse cursors.**
- ❌ **No overlapping elements.** Clean spatial separation always.
- ❌ **No 3-column equal card feature rows.** Use 2-column asymmetric or single-column.
- ❌ **No generic names** ("John Doe", "Acme", "Nexus", "Lorem Ipsum"). Realistic operator names: "Acme Capital LP", "Meridian Treasury", "Northwind Logistics."
- ❌ **No fake round numbers** (`99.99%`, `50%`). Ledger data is exact, down to the unit.
- ❌ **No AI copywriting clichés** ("Elevate", "Seamless", "Unleash", "Next-Gen", "Supercharge"). Copy should sound like an engineer wrote it for another engineer.
- ❌ **No filler UI text** ("Scroll to explore", "Swipe down", "Click here", bouncing chevrons).
- ❌ **No broken image links.** Use `picsum.photos` only for placeholder hero art; otherwise SVG or solid fills.
- ❌ **No centered hero sections** (variance 4 — asymmetric splits mandatory for docs landing surfaces).
- ❌ **No lime on lime.** Lime badge on lime CTA, lime icon on lime fill — both rejected.
- ❌ **No lime body text above 16px.** Lime in badges, KPI tiles, focus rings, 1px strokes only.
- ❌ **No lime gradients of any kind.**
- ❌ **No red.** Destructive = absence of lime, not presence of red. Errors use amber `#B45309`.

## 8. Dark Mode

Dark mode is a first-class surface, not an afterthought. The registry is sometimes viewed by operators on terminals and dim rooms; the dark surface must feel deliberate.

- **Surface inversion:** Canvas → Ink (`#0A0A0B`), Pure Surface → Raised Surface (`#161618`), Ink text → Inverted Text (`#F4F4F5`).
- **Lime stays electric.** `#D4FF3A` works on both surfaces at AAA contrast. Consider `#D9FF4D` for the dark surface if measure shows any perceived muddiness — but resist the urge to brighten lime by default.
- **Whisper borders darken, don't invert.** Zinc-700 at `0.7` opacity reads as graphite on dark surfaces, the same role Zinc-200 plays on light.
- **Status badges:** Active/Settled stays lime. Pending moves to raised-surface fill with mute text. Stale stays Ink-on-Bone — but on dark, becomes Bone-on-Raised. The semantic stays the same; the surface adapts.
- **No lime drench on dark.** The same accent discipline applies — dark mode is darker, not louder.
- **Default to system preference** (`prefers-color-scheme`). Manual toggle in user settings overrides for sessions that need explicit control.

## 9. Accessibility

`PRODUCT.md` mandates WCAG 2.1 AA minimum, AAA where cheap. The design system enforces this without exception.

- **Body text:** 4.5:1 minimum. Ink-on-Bone ≈ 17.8:1. Steel-Mute-on-Bone ≈ 7.4:1. Both pass AAA.
- **Large text:** 3:1 minimum. Headlines weight 500+ at 24px+ — Ink-on-Bone passes.
- **UI boundaries:** 3:1 minimum. Whisper Border at 0.85 opacity ≈ 1.4:1 — borderline. Where UI boundaries carry meaning (focus rings, required field indicators), use Ink at full opacity, not Whisper Border.
- **Focus indicators:** 2px Electric Lime outer ring on every interactive element via `:focus-visible`. Never remove default focus rings. Never `outline: none` without a replacement.
- **Touch targets:** 44px minimum on all interactive elements. Mobile nav, buttons, table rows, badges.
- **Color is never the sole signal.** Status badges include text. Errors include messages. The lime row indicator on selected tables is paired with a left-edge stroke + text label change.
- **Reduced motion:** Honor `prefers-reduced-motion: reduce`. Disable non-essential transitions entirely.
- **Keyboard-complete:** Every flow navigable without a mouse. Skip-link at the top of every page.
- **Screen reader:** Tables get `<caption>`, row/column headers, and `scope` attributes. Status badges get `aria-label`. Forms get `<label>` for every input.

## 10. Quick Reference (for screen generation)

When generating a new screen with Stitch, hold these in mind first:

1. **One accent.** Lime `#D4FF3A`, used for actions and state only. Never as background.
2. **Data is hero.** Tables, code, and badges carry the typographic weight. Mono everywhere data lives.
3. **Quiet motion.** 150ms ease-out. State changes only. No perpetual loops.
4. **Asymmetric splits** on docs/marketing surfaces. Generous centered content on app surfaces.
5. **Ink, not black.** `#1A1A1A` for text. `#0A0A0B` for dark canvas.
6. **No emoji, no Inter, no gradients, no rounded blobs, no bounce, no red.**
7. **Dark mode is default-able** — design the dark surface alongside the light, not after.
