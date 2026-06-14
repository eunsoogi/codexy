---
name: frontend-design
description: Use when building, redesigning, auditing, or visually verifying web apps, desktop UI, dashboards, games, onboarding, checkout, settings, plugin UI, or other user-facing product surfaces.
---

# Frontend Design

## Purpose

Make UI correct as a user experience, not just as code. A frontend change is
complete only when it fits the product domain, follows existing design
conventions, handles expected states, and has visual evidence at relevant
viewport or app sizes.

## Workflow

1. Understand context:
   - audience,
   - primary workflow,
   - information density,
   - device targets,
   - existing design system,
   - available visual assets and icon library.
2. Inspect current implementation before adding patterns.
3. Build the actual usable experience first unless the user explicitly asks for
   a landing page.
4. Choose controls by task:
   - icon buttons with tooltips for tools,
   - segmented controls for modes,
   - toggles or checkboxes for binary settings,
   - sliders, steppers, or numeric inputs for numbers,
   - menus for option sets,
   - tabs for view switching.
5. Keep layout stable with responsive constraints, fixed control dimensions,
   aspect ratios, and predictable grid tracks.
6. Avoid decorative excess: nested cards, floating-card page sections, one-note
   palettes, ornamental blobs, or oversized hero type inside compact tools.
7. Verify states: loading, empty, error, selected, hover, focus, disabled,
   keyboard, pointer, mobile, desktop.
8. Capture visual evidence and inspect for overlap, clipping, unreadable text,
   broken assets, blank canvases, inaccessible controls, and layout shift.

## Required Output

```text
Audience:
Primary workflow:
Design constraints:
Changed UI surfaces:
States covered:
Visual QA evidence:
Known limits:
```

## Gates

- Do not introduce a new visual language without checking existing components.
- Do not rely on code review alone for UI correctness.
- Do not ship text that overflows, overlaps, or hides controls at supported
  sizes.
- Do not claim a canvas, image, or media surface works without visual evidence.

## Evidence Rules

- Desktop and mobile screenshots are required when responsive layout changes.
- Interactive controls need action evidence, not just initial render evidence.
- Asset changes need proof that referenced files load in the target surface.
- Accessibility or keyboard claims need direct keyboard/focus observation.

## Failure Modes

- Building a marketing page when the user asked for an app or tool.
- Using generic decorative layouts that do not fit operational software.
- Letting dynamic labels resize fixed-format controls.
- Forgetting empty, error, loading, and disabled states.
