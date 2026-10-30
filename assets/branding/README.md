# Agent Browser brand

The approved mark is a teal interwoven rosette wearing a black secret-agent
fedora and sunglasses, with a soft teal glow. It identifies Agent Browser in
the browser tab, dashboard and documentation, helping operators recognize the
browser workspace when switching between task and handoff tabs.

`agent-browser-logo.png` is the original approved transparent PNG, generated
with the built-in image generation tool and adopted by the operator on
October 30, 2026. Preserve its proportions and transparent negative space.
The public header images are 256-pixel exports. Favicons contain 16, 32, 48
and 64-pixel frames. Apple touch icons are 180 pixels. Icon exports use a light
slate background so the black hat and glasses remain visible on dark browser
chrome. The glow is baked into the artwork; no animation or extra CSS glow is
needed.

To reproduce exports with ImageMagick from the repository root:

```sh
convert assets/branding/agent-browser-logo.png -resize 256x256 docs/public/branding/agent-browser-logo.png
cp docs/public/branding/agent-browser-logo.png packages/dashboard/public/branding/agent-browser-logo.png
convert assets/branding/agent-browser-logo.png -background '#f8fafc' -alpha remove -alpha off -define icon:auto-resize=64,48,32,16 docs/src/app/favicon.ico
cp docs/src/app/favicon.ico packages/dashboard/src/app/favicon.ico
convert assets/branding/agent-browser-logo.png -background '#f8fafc' -alpha remove -alpha off -resize 180x180 docs/public/apple-touch-icon.png
cp docs/public/apple-touch-icon.png packages/dashboard/public/apple-touch-icon.png
```

Generation prompt direction: preserve the interwoven rosette, tilted fedora and
sunglasses; change the rosette to rich luminous teal with a restrained soft
glow, keep the hat and glasses near-black, and retain transparent gaps and
background. No text or additional objects.

Work item: [493](https://github.com/CochranResearchGroup/agent-browser/issues/493).
This adoption changes source presentation only; production installation is a
separate operation.

Validation: documentation and dashboard production builds (including TypeScript)
pass, as do policy wiring, documentation links and patch hygiene. Both favicon
sets contain the four documented sizes; the dashboard static export includes
the logo, favicon and touch icon with matching HTML metadata. The 16-pixel
favicon was inspected visually. No installed browser or service was changed.
