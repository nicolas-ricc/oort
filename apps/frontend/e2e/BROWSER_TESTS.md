# Browser Test Checklist (Playwright MCP)

Interactive smoke tests executed via Playwright MCP tools. Not a CI runner — these are step-by-step procedures for visual verification.

## Prerequisites

Start the dev server before running tests:

```bash
# Frontend only (sufficient for T1-T5)
cd apps/frontend && pnpm dev

# Full stack (required for T6-T8)
docker compose up
```

Frontend serves at `http://localhost:5173`.

---

## T1 — App Loads (Empty State)

1. `browser_navigate` -> `http://localhost:5173`
2. `browser_snapshot` -> verify "Upload Text File" button visible
3. Verify canvas element exists (WebGL context)
4. Verify no 70/30 layout split — canvas fills viewport

## T2 — Terminal Overlay Toggle

1. `browser_press_key` -> Backquote (`` ` ``)
2. `browser_snapshot` -> verify "Search concepts..." input visible
3. Verify file upload input exists inside terminal
4. Verify URL link icon button present
5. `browser_press_key` -> Escape
6. `browser_snapshot` -> verify terminal gone (no "Search concepts..." input)

## T3 — Terminal Backdrop Click Close

1. `browser_press_key` -> Backquote
2. `browser_snapshot` -> terminal visible
3. `browser_click` on backdrop area (outside terminal panel)
4. `browser_snapshot` -> terminal closed

## T4 — HUD Hidden When Empty

1. `browser_snapshot` -> verify no "Speed", "Region", "Discovered" text
2. Verify no minimap canvas element

## T5 — Empty State Modal Interaction

1. `browser_snapshot` -> "Upload Text File" button visible
2. Verify file input with `.txt,.md,.text` accept attribute exists

## T6 — Post-Upload Verification (requires ML backend)

1. Upload a small text file via empty state modal
2. Wait for processing to complete
3. `browser_snapshot` -> verify:
   - Planets section exists (concept names in accessibility tree)
   - HUD elements: "Speed", compass heading, "Discovered X/X"
   - Minimap canvas at bottom-left
   - Controls hint: "WASD fly", "Mouse look", etc.
4. `browser_press_key` -> Backquote
5. `browser_snapshot` -> terminal overlay shows concept list

## T7 — Keyboard Navigation (requires data loaded)

1. `browser_press_key` -> "n" (navigate to next planet)
2. `browser_snapshot` -> floating panel appears with concept name
3. `browser_press_key` -> Escape (deselect)
4. `browser_snapshot` -> floating panel gone

## T8 — Scene Persistence (requires data loaded)

1. Open terminal, click Save button
2. Verify URL updated with `?scene=` parameter
3. `browser_navigate` -> reload the URL
4. `browser_snapshot` -> concepts still loaded
