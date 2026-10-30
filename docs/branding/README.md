# Rivetr logo

The indigo tile and white R monogram combine a rivet/join detail with the forward motion of deploying code. The mark works on light and dark surfaces; pair it with the existing Geist wordmark in the app.

- `rivetr-logo-source.png`: original transparent image, generated with the built-in imagegen tool.
- `../../frontend/public/brand/rivetr-mark.png`: cropped, 512px production mark with transparency preserved.
- `../../frontend/public/favicon.ico`: 16, 32, 48, 64, 128 and 256px icon frames.
- `../../frontend/public/apple-touch-icon.png`: 180px touch icon.

The shared `RivetrLogo` and `RivetrBrand` components apply the mark across the app and honor configured white-label logos and names. Browser favicon overrides remain supported.

## Generation prompt

```text
Use case: logo-brand
Asset type: production logo symbol for Rivetr, a lightweight self-hosted Git deployment platform.
Primary request: Design one distinctive, minimal geometric R monogram that feels like interlocking infrastructure with a forward-rising diagonal leg, a subtle rivet/join concept. Bold silhouette, generous negative space, expertly balanced. This will be displayed at 24px as well as large.
Style/medium: flat vector-style logo artwork, crisp precise edges, no 3D. A single indigo rounded-square tile with a bold white abstract R cut into it, subtly softened outer corners. Understated professional developer tooling identity.
Composition/framing: one centered symbol only filling about 85 percent of a square image, equal small clear margins on all sides. All outside the indigo tile genuinely transparent.
Color palette: solid indigo #4f46e5 and white only. White monogram must be opaque white.
Text: no wordmark or text; the abstract R monogram is the only glyph.
Constraints: one logo only, no presentation sheet, no mockup, no tiny details, no hairline strokes, no gradient, no shadow, no watermark. Actual alpha transparency outside tile.
```
