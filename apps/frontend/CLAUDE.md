# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

React + TypeScript frontend for Oort concept visualization. Uses React Three Fiber to render concepts as planets in 3D space. The app starts empty and prompts the user to upload text files, which are processed into concept clusters and displayed as an interactive 3D mind-map.

## User Journey

### 1. Empty State (first launch)
The app starts with **no data** (`simulationData` is an empty array). The 3D canvas renders full-screen showing only the starfield background. An `EmptyStateModal` overlays the scene — a terminal-themed card prompting the user to upload a text file.

### 2. File Upload
The user clicks "Upload Text File" in the modal (or later via the upload icon in the Menu). The file is read client-side, sent to `POST /api/vectorize`, and the backend returns `ConceptCluster[]` data. During processing, a blinking loading indicator is shown. On success, `simulationData` is populated and `isEmpty` flips to `false`.

### 3. Visualization
Once data exists, the modal unmounts. The layout switches to a 70/30 split: the 3D canvas (top 70%) renders planets from concept clusters, and the Menu (bottom 30%) shows a searchable concept list with file upload. The camera auto-focuses on the first uploaded cluster's planet.

### 4. Planet Selection
Clicking a planet in the 3D scene sets it as `active`. The camera animates to frame the planet and its nearby cluster. After the animation completes, a `FloatingPlanetPanel` appears as a DOM overlay near the planet's screen position. The panel shows:
- Cluster color dot + primary concept label
- Numbered list of all concepts in the cluster
- Source text references (fetched via `GET /api/texts-by-concept`)

The panel tracks the planet's projected screen position every frame via `requestAnimationFrame`, flipping sides if it would overflow the viewport edge.

### 5. Deselection
The user can dismiss the floating panel by:
- Clicking the X button on the panel
- Clicking the canvas background (`onPointerMissed`)
- Pressing ESC (resets to overview camera)

### 6. Navigation
Keyboard shortcuts allow navigating between planets:
- Arrow keys / N / P: next/previous planet
- T: toggle auto-tour mode
- 1-9: jump to planet by index
- ESC: reset to overview

Users can also search and click concepts in the Menu to navigate to specific planets.

## Development Commands

```bash
pnpm dev        # Dev server at :5173
pnpm build      # TypeScript check + Vite build
pnpm lint       # ESLint
pnpm test       # Run Vitest unit tests
pnpm preview    # Preview production build
pnpm ri         # Clean reinstall (rm node_modules + pnpm-lock.yaml + install)
```

## Component Hierarchy

```
App.tsx
├── QueryClientProvider (TanStack Query)
├── Layout (isEmpty, canvasRef)
│   ├── Render.tsx (Canvas + camera controls + screenPositionRef)
│   │   └── Scene.tsx (3D scene + screen projection)
│   │       └── Planet.tsx (individual concept nodes)
│   └── Menu.tsx (concept search + file upload) [hidden when isEmpty]
├── FloatingPlanetPanel (fixed overlay, tracks planet screen position)
└── EmptyStateModal (shown when simulationData is empty)
```

## Key Files

- `App.tsx` - Top-level state (`simulationData`, `active`, `isAnimating`), refs (`screenPositionRef`, `canvasRef`), empty/populated branching
- `cloud/Scene.tsx` - 3D scene, collision avoidance, planet rendering, `useFrame` screen projection of active node
- `cloud/Planet.tsx` - Planet mesh with texture and floating label
- `cloud/Render.tsx` - R3F Canvas setup, `onPointerMissed` deselection, passes `screenPositionRef` and `onAnimatingChange` to parent
- `layout/Layout.tsx` - Responsive shell: full-height canvas when `isEmpty`, 70/30 split otherwise
- `layout/Menu.tsx` - Terminal-styled command palette with concept search and file upload icon
- `layout/FloatingPlanetPanel.tsx` - DOM overlay positioned via `screenPositionRef`, shows concepts + source texts for selected planet
- `layout/EmptyStateModal.tsx` - Full-screen modal for first-time upload, uses `useFileUpload` hook
- `hooks/useFileUpload.ts` - Shared TanStack Query mutation for file upload (used by Menu and EmptyStateModal)
- `hooks/useNavigation.ts` - Keyboard/programmatic navigation between planets

## 3D-to-Screen Projection (FloatingPlanetPanel)

The floating panel positions itself near the selected planet without living inside the 3D scene:

1. **Scene.tsx** uses `useFrame` to project `activeNodePosition` to screen pixels via `camera.project()` + canvas dimensions, writing to `screenPositionRef.current`
2. **App.tsx** owns the `screenPositionRef` and passes it down to both `Render` (which passes to `Scene`) and `FloatingPlanetPanel`
3. **FloatingPlanetPanel** runs a `requestAnimationFrame` loop reading `screenPositionRef.current` and updating its `position: fixed` CSS. It offsets 30px to the right of the planet, vertically centered, and flips to the left side if overflowing the viewport edge

The panel only renders when `!isAnimating && selectedCluster !== null`, preventing it from showing during camera transitions.

## Critical: SCENE_SCALE Synchronization

`SCENE_SCALE = 2` is defined in multiple files and MUST stay synchronized:
- `App.tsx:11`
- `cloud/hooks/useSceneScale.ts` (canonical source, imported by Scene.tsx and Render.tsx)

This constant controls all 3D positioning: planet distances, camera position, collision thresholds.

## State Management

- **TanStack Query** - API calls to ML backend (`/api/vectorize`, `/api/texts-by-concept`)
- **Local state in App.tsx**:
  - `simulationData: Simulation` - array of concept clusters (starts empty)
  - `active: string` - selected node key (scaled embedding coords joined with "-"), empty string = no selection
  - `isAnimating: boolean` - true during camera transitions, suppresses floating panel
  - `isLoading: boolean` - true during file upload processing
- **Refs**:
  - `screenPositionRef` - mutable ref updated every frame with active planet's {x, y} screen position
  - `canvasRef` - ref to the canvas container div
- **Node identification** - Nodes identified by scaled embedding coordinates joined with "-"
- **Empty state** - `isEmpty = simulationData.length === 0` drives layout mode and modal visibility

## API Integration

```typescript
// POST /api/vectorize
{ text: string, user_id?: string, filename?: string }
// Returns: { success: boolean, data: ConceptCluster[] }

// GET /api/texts-by-concept?concept=X&user_id=Y
// Returns: { success: boolean, data: TextReference[] }
```

## 3D Coordinate System

- Embeddings from backend are 3D coordinates from PCA reduction
- Coordinates are scaled by `SCENE_SCALE` for rendering
- `avoidCollisions()` in Scene.tsx pushes overlapping planets apart
- Planet radius is 1 unit, minimum safe distance is `(4 + 2) * SCENE_SCALE`

## Post-Processing (EffectComposer)

When using `@react-three/postprocessing` with React Three Fiber, be aware of render loop issues.

### Problem

Scene goes pitch black when idle, only renders during camera movement or interaction.

### Root Cause

EffectComposer from `@react-three/postprocessing` can interfere with R3F's render loop, causing the scene to stop rendering when there are no state updates.

### Solution

Add these props to EffectComposer:

```tsx
import { HalfFloatType } from 'three'

<EffectComposer
  multisampling={0}           // Disable MSAA which can cause render stalls
  disableNormalPass={true}    // Skip unnecessary normal pass for simple effects
  frameBufferType={HalfFloatType}  // Better color precision
>
  {/* effects */}
</EffectComposer>
```

Also ensure the Canvas has continuous rendering enabled:

```tsx
<Canvas frameloop="always">
  {/* scene content */}
</Canvas>
```

### Why This Works

- `multisampling={0}` - MSAA can cause the render pipeline to stall when combined with post-processing
- `disableNormalPass={true}` - The normal pass is only needed for effects like SSAO; skipping it reduces complexity
- `frameBufferType={HalfFloatType}` - Uses 16-bit floats for better precision and compatibility
- `frameloop="always"` - Forces continuous rendering instead of on-demand, ensuring post-processing effects always have fresh frames to composite

## Unit Tests

Run tests with `pnpm test`. Test files use Vitest + React Testing Library.

### Menu Component Tests (`layout/Menu.test.tsx`)

**Unit Tests:**
| Test | Coverage |
|------|----------|
| `renders the file upload input` | File input exists with correct type and accept attributes |
| `renders concept list when concepts are provided` | Concept names display correctly |
| `calls onSelect when a concept is clicked` | Click handler triggers with concept name |
| `shows search input placeholder` | Search input has correct placeholder |
| `shows "No results found" for empty concepts list` | Empty state displays correctly |

**File Upload - Loading State:**
| Test | Coverage |
|------|----------|
| `calls setLoadingState(true) when file upload starts` | Loading starts on upload |
| `calls setLoadingState(false) after file upload completes` | Loading ends on success |
| `calls setLoadingState(false) even when upload fails` | Loading ends on error |

**File Upload - API Integration:**
| Test | Coverage |
|------|----------|
| `sends correct payload to API on file upload` | POST body has user_id, text, filename |
| `calls onSimulationUpdate with API response data` | Callback receives concept data |
| `shows alert and logs error when API call fails` | Error handling works |
| `does not make API call when no file is selected` | Empty selection is ignored |

### Test Utilities

- `test/test-utils.tsx` - Custom render with QueryClientProvider
- Uses `vi.mocked(fetch)` for API mocking
- `userEvent.setup()` for user interaction simulation
