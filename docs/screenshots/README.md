# Screenshots

Native Windows captures from the rebuilt application on 2026-10-15 (JPEG output
from the Windows capture helper, at its actual capture size):

- [Narrow relationship map](hardening-map-narrow-dark.jpg)
- [Narrow Settings and local storage](hardening-settings-narrow-dark.jpg)

These use the existing local forecast-service fixture index, dated 2026-10-08.
They show the current application reading an existing index, not a fresh indexing
benchmark. Neither screenshot is a generated mockup.

## Historical frontend captures

Current Threadmark interface, captured at 2× pixel density in lossless PNG format.
The wide views are 2880 × 1800 pixels; narrow views are 1600 × 1400.

The frontend is rendered in Chromium using responses recorded from Threadmark's
Rust engine against local example repositories. Source, relationships, diagnostics
and Git comparison results come from those indexes. These captures show the UI;
native Windows verification is described in [release notes](../release.md).

- [Welcome](welcome-dark.png)
- [Relationship map](map-dark.png)
- [Source evidence](evidence-dark.png)
- [Git changes](changes-dark.png)
- [Settings](settings-light.png)
- [Change impact](impact-light.png)
- [Related tests](tests-light.png)
- [Diagnostics](diagnostics-light.png)
- [Narrow map](map-narrow-dark.png)
- [Inspector drawer](inspector-narrow-dark.png)
- [Full-name neighborhood list](neighborhood-list-dark.png)
