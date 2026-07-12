# Dashboard Visual Baseline

Status: approved

Approval date: 2026-07-12

Applies to delivery increment 5 and every later surface that renders the
control-plane read model.

## Decision

The Dashboard uses the approved **Command Grid** information architecture with
the **Moonlight Indigo** color system, **system sans-serif** typography, and
complete **zh-CN / en-US** localization.

The user also approved a larger compact-type scale: operational body text is
13px equivalent, secondary labels are 12px, and machine metadata is never below
11px before browser zoom or accessibility scaling.

This decision closes the visual-direction gate. Later implementation may tune
spacing or component mechanics to satisfy accessibility and responsive
constraints, but it must not change the layout hierarchy, default palette,
typographic category, bilingual contract, or minimum type scale without new
user evidence.

## Information Architecture

The work-package view remains primary and session topology remains secondary.
The desktop composition is:

1. compact global top bar with run identity, breadcrumb, and factual run state;
2. narrow left navigation for work, sessions, events, routing, token economy,
   and quality gates;
3. dense summary strip for objective, package progress, assignment progress,
   complete token cost, and quality state;
4. two-column operational region:
   - wide column: work-package dependency map followed by event timeline;
   - narrow column: agent dock followed by token-economy breakdown;
5. no decorative hero cards, oversized empty regions, or observer-generated
   facts.

At narrower widths, the operational columns stack while the work map remains
first. Navigation collapses to icons before content density is reduced.

## Moonlight Indigo Tokens

The default theme is a light, cool-neutral liquid-glass surface:

| Token | Value | Purpose |
|---|---|---|
| canvas-primary | #e8edf5 | primary page field |
| canvas-secondary | #d9e1ed | depth gradient and lower field |
| glass-surface | rgba(251, 253, 255, 0.72) | navigation and grouped panels |
| glass-opaque | rgba(246, 249, 252, 0.96) | fallback and evidence surfaces |
| text-primary | #172236 | operational text |
| text-secondary | #596981 | secondary labels |
| text-tertiary | #637288 | non-critical metadata |
| border-subtle | rgba(44, 62, 91, 0.12) | panel separation |
| border-strong | rgba(44, 62, 91, 0.21) | active grouping and shell edge |
| accent-indigo | #5968d7 | active state, selection, and focus |
| state-ready | #2f64bd | ready or queued |
| state-active | #5968d7 | active work |
| state-complete | #16724f | accepted or complete |
| state-warning | #8a570b | attention without failure |
| state-blocked | #b13b49 | blocked or failed |

Semantic colors are invariant across locales. Color never carries state alone;
every state includes text and, where useful, an icon or shape.

Translucency and blur communicate grouping and depth. Evidence tables, logs,
long-form status, and dense text use sufficiently opaque surfaces. The
Dashboard has an equivalent glass-opaque rendering when backdrop filtering is
unavailable or reduced transparency is requested.

## Typography

The interface uses system sans-serif fonts rather than a bundled decorative
face. Locale-aware stacks are:

    :lang(zh-CN) {
      font-family:
        "PingFang SC",
        "Microsoft YaHei",
        "Noto Sans CJK SC",
        "Droid Sans Fallback",
        system-ui,
        sans-serif;
    }

    :lang(en-US) {
      font-family:
        system-ui,
        -apple-system,
        BlinkMacSystemFont,
        "Segoe UI",
        sans-serif;
    }

IDs, session handles, model names, token values, counters, hashes, and
timestamps use a system monospace stack. Monospace is not used for Chinese
prose or navigation labels.

The approved compact scale is:

| Use | Minimum |
|---|---|
| operational body and table content | 13px |
| navigation and secondary labels | 12px |
| IDs, timestamps, counters, and machine metadata | 11px |
| primary panel title | 14px |
| run or objective title | 17px |

Important state and action labels must not rely on font weight below the
available CJK rendering quality.

## Bilingual Product Contract

The Dashboard ships with complete zh-CN and en-US interfaces.

- The user can switch languages without a reload.
- The initial locale follows the persisted user preference, then the host
  locale, then falls back to en-US.
- All user-facing strings live in locale resources; components do not
  concatenate translated fragments.
- Dates, durations, counts, and pluralization are locale-aware. Stable IDs,
  host names, model identifiers, hashes, and event field names remain unchanged.
- Unknown telemetry renders as the localized form of unknown; it is never
  converted into zero or success.
- Chinese and English layouts use the same information hierarchy and available
  functionality.
- Longer English labels and dense Chinese labels do not clip, overlap, or hide
  evidence. Tooltips are not the only way to discover required information.
- Browser and CLI locale choices are independent presentation preferences over
  the same durable facts.

An observer agent is not used for translation. Localization resources and
deterministic formatters produce interface text.

## Motion and Accessibility

- WCAG AA contrast is mandatory for both locales and all semantic states.
- Keyboard focus uses the indigo accent plus a visible outline.
- Motion is limited to factual lifecycle changes and direct user interaction.
- Reduced-motion preference removes non-essential transitions.
- Reduced-transparency mode and the no-backdrop-filter fallback use opaque
  surfaces without losing hierarchy or state.
- Hover is never required for core operation.
- Work-map relationships have a text or table equivalent for screen readers and
  narrow screens.

## Acceptance Evidence

Increment 5 is not complete until:

1. representative Work Map, Agent Dock, Token Economy, and Event Timeline views
   match this hierarchy and token system;
2. automated contrast checks pass for both locales and every semantic state;
3. screenshot coverage includes zh-CN and en-US at desktop and compact
   breakpoints, opaque fallback, and reduced-motion or transparency modes;
4. keyboard and screen-reader checks cover navigation, language switching,
   package selection, session inspection, filters, and evidence links;
5. locale-resource tests reject missing or unused keys and verify deterministic
   number, date, and duration formatting;
6. no Dashboard state depends on an observer LLM;
7. the user-approved baseline is represented by this document, so a new
   aesthetic approval is unnecessary unless implementation evidence forces a
   material departure.
