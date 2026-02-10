# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

React + TypeScript frontend for Oort concept visualization. Uses React Three Fiber to render concepts as planets in 3D space.

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
├── Layout
│   ├── Render.tsx (Canvas + camera controls)
│   │   └── Scene.tsx (3D scene)
│   │       └── Planet.tsx (individual concept nodes)
│   └── Menu.tsx (text input + concept search)
```

## Key Files

- `cloud/Scene.tsx` - Main 3D scene, collision avoidance, planet positioning
- `cloud/Planet.tsx` - Planet mesh with texture and floating label
- `cloud/Render.tsx` - R3F Canvas setup, OrbitControls, post-processing
- `layout/Menu.tsx` - Command palette for text input and concept search
- `App.tsx` - State management, simulation data handling

## Critical: SCENE_SCALE Synchronization

`SCENE_SCALE = 2` is defined in multiple files and MUST stay synchronized:
- `App.tsx:9`
- `cloud/Scene.tsx:18`
- `cloud/Render.tsx` (camera positioning)

This constant controls all 3D positioning: planet distances, camera position, collision thresholds.

## State Management

- **TanStack Query** - API calls to ML backend
- **Local state** - `simulationData` (concept clusters), `active` (selected node ID)
- **Node identification** - Nodes identified by scaled embedding coordinates joined with "-"

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
