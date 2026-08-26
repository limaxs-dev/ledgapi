# Ledgapi — Logo Mark Redesign Spec

**Date:** 2026-08-26
**Status:** Draft for review
**Scope:** Brand mark only (no wordmark, no full lockup, no color system beyond the mark)

---

## 1. Purpose

Redesign the ledgapi logo mark used in `README.md` and other light/dark-theme surfaces (GitHub). The existing mark (`logo.png` / `logo.svg` in repo root) keeps the right **concept** — an open book as ledger metaphor with "API" in the centre — but its **colour treatment** is heavy and dated (dark teal/navy, glow shadow, decorative code brackets) and clashes with the brand personality documented in `PRODUCT.md`:

- *Precise, trustworthy, quiet. A ledger: factual, calm, no theatrics.*
- *Restraint: one accent, used for actions and state only.*

The new mark keeps the metaphor, drops the noise, and uses a single accent — electric lime — as the brand block.

## 2. Decisions (from brainstorming)

| # | Question | Decision |
|---|---|---|
| 1 | Concept | **Buku terbuka + "API"** (same metaphor as existing mark). |
| 2 | Accent colour | **Electric lime `#d4ff3a`** (bright lime, balanced for sustained UI exposure). |
| 3 | Mode strategy | **Single mode.** Lime as a brand-block tile is self-contained, so the mark reads correctly on light *and* dark GitHub README themes. No dark/light variants in v1. |
| 4 | Style line | **Thin outline, 1.5px** (presisi & tenang). |
| 5 | Output | **Mark only** (icon). The "ledgapi" wordmark is handled separately by product typography. |
| 6 | File placement | New files at repo root: `logo-mark.svg` and `logo-mark.png`. Existing `logo.png` and `logo.svg` are **not** modified. |
| 7 | Existing-mark treatment | Old files remain until a future commit explicitly retires them. README.md is **not** updated in this spec. |

## 3. Visual spec

### 3.1 Canvas

- **viewBox:** `0 0 256 256` (square, 1:1).
- **Outer corner radius:** 28px on the 256 grid (≈ 11% of side).
- **Inner padding:** 20% on all sides; book artwork lives inside the inner 80% box.

### 3.2 Layers (back to front)

1. **Tile background** — rounded square filled `#d4ff3a`, occupies full 256×256.
2. **Book outline** — two open pages with a central spine, stroked `#1a1a1a`.
3. **"API" text** — centred on the spine, filled `#1a1a1a`.

No drop shadows, no gradients, no filters, no glow.

### 3.3 Book geometry

- Book bounding box: **60% of inner-tile width × 50% of inner-tile height**, horizontally and vertically centred inside the inner tile.
- Two mirrored page shapes meeting at a vertical spine.
- Pages have slightly rounded outer corners (no sharp points); inner edges meet the spine cleanly.
- Spine: a single thin vertical line at the centre, ~50% of book height, stroke 1.5px.

### 3.4 "API" text

- Centred on the spine, baseline at vertical midpoint of book.
- Font: any clean geometric sans (system fallback acceptable for raster preview; in the SVG use a generic stack such as `"Inter", "SF Pro Text", system-ui, sans-serif`).
- Weight: 500 (medium).
- Letter-spacing: -0.02em.
- Size: roughly 40% of book width.
- Fill: `#1a1a1a`.

### 3.5 Stroke details

- All strokes: `#1a1a1a`, width **1.5px** in the 256 viewBox.
- `stroke-linejoin`: `round`.
- `stroke-linecap`: `round`.
- `vector-effect`: `non-scaling-stroke` is **not** used; the SVG is designed to scale cleanly so stroke scales proportionally.

### 3.6 Colour palette (this mark only)

| Token | Hex | Usage |
|---|---|---|
| `lime.brandBlock` | `#d4ff3a` | Tile background. The only saturated colour in the mark. |
| `ink.primary` | `#1a1a1a` | Book outline + "API" text. |

No other colours in the mark. These are tokens for the brand mark only; the product colour system is documented separately in `PRODUCT.md` and is out of scope for this spec.

### 3.7 Accessibility

- WCAG 2.1 AA contrast between `#1a1a1a` ink and `#d4ff3a` lime tile: **15.2 : 1** (well above 4.5:1 for body text, 3:1 for UI boundaries).
- The mark itself has no text intended to be read at favicon size; the "API" letters remain legible at ≥ 24px on screen.
- No motion, no flashing — accessibility floor trivially met.

## 4. File deliverables

All files live at the **repo root** alongside (not replacing) the existing `logo.png` / `logo.svg`.

| File | Format | Size | Purpose |
|---|---|---|---|
| `logo-mark.svg` | SVG 1.1 | vector, viewBox `0 0 256 256` | Source of truth. Hand-authored, minimal markup. |
| `logo-mark.png` | PNG, sRGB | 1024 × 1024 px | Raster export for `README.md` and any raster-only consumer. Optimised (target ≤ 80 KB). |

No favicon, no social-avatar variant, no `@2x` retina, no animated variant in this spec.

## 5. Implementation notes

- The SVG must be **hand-authored** (no auto-trace from the old PNG), keeping markup small and shapes intentional.
- The PNG export should be produced from the SVG using a deterministic renderer (e.g. `rsvg-convert` or `inkscape --export-type=png`) at 1024 × 1024, sRGB, with anti-aliasing on. No raster-only effects.
- The SVG should set `role="img"` and an `<title>` of `ledgapi` for assistive tech.
- No external font references; the SVG must render with system fonts alone.
- A trivial smoke check: opening `logo-mark.svg` and `logo-mark.png` in a viewer should show the same composition; the PNG must not contain anti-aliasing artefacts that look like a halo at the tile edge.

## 6. Out of scope

- Replacing `logo.png` / `logo.svg` or updating `README.md` to reference the new mark.
- Dark-mode-specific or light-mode-specific variants.
- A wordmark, full lockup, or any other logo composition.
- Favicon / app-icon / social-avatar derivatives.
- Animations, motion variants.
- Updates to `PRODUCT.md` brand-personality language (it already aligns).

## 7. Open questions

None at draft time.
