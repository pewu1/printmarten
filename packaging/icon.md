# PrintMarten icon

The application uses `icon.png` as its transparent master artwork. The original
`icon.jpg` is retained as the design reference. `src/branding.rs` embeds the PNG
for the window and toolbar; `scripts/package.py` converts it to macOS ICNS,
Windows ICO, and Linux PNG assets. `packaging/printmarten.ico` is the matching
multi-size icon embedded in the Windows executable by `build.rs`.

Artwork was edited using the built-in imagegen tool, with these prompts:

## Initial refinement

Use case: precise-object-edit / logo-brand. Edit target: the attached existing PrintMarten application icon. Rework it into a polished production application icon that remains clearly the same warm cream-and-copper marten mascot inside a dark charcoal circular badge with a cream rim. Keep its recognizable front-facing marten head, big dark eyes with small highlights, pale face, tan forehead, copper/orange neck, upright ears, geometric illustration character, and original warm palette. Improve optical balance and symmetry, simplify the many narrow facial facets and thin whiskers into fewer bold readable shapes, give face and ears a clean confident silhouette, increase contrast slightly, and use crisp antialiased flat vector-like edges. Head should dominate the badge so it remains recognizable at 24 and 32 pixels. Rim must be even and fully visible, whole circular badge centered with approximately 6 percent transparent padding on every side, with absolutely no crop at the canvas edges. Clean perfectly square 1024 by 1024 icon canvas. Genuine transparent background outside the circular badge, no white corner wedges, no shadows outside the badge. Do not add a printer, letters, lettering, words, watermark, multiple variants, mockup, or unrelated objects. One finished icon only.

## Final framing refinement

Refine this exact icon; preserve the marten design, palette, facial features, eye placement, and the circular cream rim. Change only the framing and edge cleanup: the complete badge must be a clean geometrically smooth perfect circle, centered and scaled down to occupy 80 percent of a square transparent canvas (10 percent completely transparent padding on all four sides). Remove every tiny white speck, fringe, rough fleck, stray pixel, and fragment outside the circular badge. The outer silhouette must be a perfectly smooth clean antialiased circle against genuine alpha transparency. No white backdrop. The transparent margin must remain completely empty. Do not add any objects, text, mockups, shadows or glow. This is a polished desktop app icon used at 24px and 32px. Keep the reference artwork as close as possible within the clean circular edge. One finished square icon.
