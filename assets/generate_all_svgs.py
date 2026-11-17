"""
Generate crisp, scalable vector graphics (SVG) for the 4 Yav logos in yav.rs/assets.
"""

import os

# Resolve relative to this script so the generator works on any machine.
SVG_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "svg")
os.makedirs(SVG_DIR, exist_ok=True)

# ---------------------------------------------------------------------------
# 1. Monogram Flat (yav-monogram-flat.svg)
# ---------------------------------------------------------------------------
SVG_MONOGRAM_FLAT = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <!-- Yav Monogram - Flat Icon -->
  <g id="yav-monogram-flat" fill="#F5A623">
    <!-- Left Wing -->
    <path d="M 56.4 69.4
             L 57.0 73.9
             L 179.0 249.2
             L 181.2 248.7
             L 238.6 176.7
             L 178.4 90.2
             C 170.5 81.5 159.0 73.0 144.7 70.5
             Z" />
    
    <!-- Center Main Diagonal Wing -->
    <path d="M 442.1 49.1
             C 418.0 48.0 380.0 51.5 360.6 60.4
             L 118.3 356.6
             C 112.5 363.8 108.7 372.9 108.7 372.9
             L 163.2 371.8
             L 188.0 360.0
             L 442.1 49.1
             Z" />
    
    <!-- Lower Right Parallel Wing -->
    <path d="M 454.4 152.6
             C 430.0 151.5 394.0 156.0 374.0 165.5
             L 132.9 461.8
             L 183.5 462.9
             C 194.5 459.5 208.0 452.0 216.7 447.1
             Z" />
  </g>
</svg>"""

# ---------------------------------------------------------------------------
# 2. Monogram Gradient (yav-monogram-gradient.svg)
# ---------------------------------------------------------------------------
SVG_MONOGRAM_GRADIENT = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Main fiery gradient: luminous yellow to deep orange/coral -->
    <linearGradient id="yav-gold-fire" x1="85%" y1="0%" x2="15%" y2="100%">
      <stop offset="0%" stop-color="#FFDD44" />
      <stop offset="25%" stop-color="#FFA812" />
      <stop offset="65%" stop-color="#FF6B00" />
      <stop offset="90%" stop-color="#FF3800" />
      <stop offset="100%" stop-color="#E01E00" />
    </linearGradient>

    <!-- Top-left wing warm ambient gradient -->
    <linearGradient id="yav-left-wing" x1="10%" y1="0%" x2="90%" y2="100%">
      <stop offset="0%" stop-color="#FFDD55" />
      <stop offset="40%" stop-color="#FFAA15" />
      <stop offset="100%" stop-color="#FF5D00" />
    </linearGradient>

    <!-- Lower wing gradient -->
    <linearGradient id="yav-lower-wing" x1="90%" y1="10%" x2="10%" y2="90%">
      <stop offset="0%" stop-color="#FF9E00" />
      <stop offset="50%" stop-color="#FF5500" />
      <stop offset="100%" stop-color="#DC1414" />
    </linearGradient>

    <!-- Subtle specularity overlay -->
    <linearGradient id="yav-sheen" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.35" />
      <stop offset="35%" stop-color="#FFFFFF" stop-opacity="0.0" />
      <stop offset="100%" stop-color="#000000" stop-opacity="0.25" />
    </linearGradient>

    <!-- Drop shadow for depth on dark or light interfaces -->
    <filter id="yav-depth" x="-10%" y="-10%" width="125%" height="125%">
      <feDropShadow dx="0" dy="12" stdDeviation="16" flood-color="#FF4500" flood-opacity="0.3" />
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#000000" flood-opacity="0.25" />
    </filter>
  </defs>

  <g id="yav-monogram-gradient" filter="url(#yav-depth)">
    <!-- Left Wing -->
    <path d="M 56.4 69.4
             L 57.0 73.9
             L 179.0 249.2
             L 181.2 248.7
             L 238.6 176.7
             L 178.4 90.2
             C 170.5 81.5 159.0 73.0 144.7 70.5
             Z"
          fill="url(#yav-left-wing)" />
    <!-- Left Wing Sheen -->
    <path d="M 56.4 69.4
             L 57.0 73.9
             L 179.0 249.2
             L 181.2 248.7
             L 238.6 176.7
             L 178.4 90.2
             C 170.5 81.5 159.0 73.0 144.7 70.5
             Z"
          fill="url(#yav-sheen)" />

    <!-- Center Main Diagonal Wing -->
    <path d="M 442.1 49.1
             C 418.0 48.0 380.0 51.5 360.6 60.4
             L 118.3 356.6
             C 112.5 363.8 108.7 372.9 108.7 372.9
             L 163.2 371.8
             L 188.0 360.0
             L 442.1 49.1
             Z"
          fill="url(#yav-gold-fire)" />
    <!-- Center Sheen -->
    <path d="M 442.1 49.1
             C 418.0 48.0 380.0 51.5 360.6 60.4
             L 118.3 356.6
             C 112.5 363.8 108.7 372.9 108.7 372.9
             L 163.2 371.8
             L 188.0 360.0
             L 442.1 49.1
             Z"
          fill="url(#yav-sheen)" />

    <!-- Lower Right Parallel Wing -->
    <path d="M 454.4 152.6
             C 430.0 151.5 394.0 156.0 374.0 165.5
             L 132.9 461.8
             L 183.5 462.9
             C 194.5 459.5 208.0 452.0 216.7 447.1
             Z"
          fill="url(#yav-lower-wing)" />
  </g>
</svg>"""

# ---------------------------------------------------------------------------
# 3. Hexagon Badge (yav-hex-badge.svg)
# ---------------------------------------------------------------------------
SVG_HEX_BADGE = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Outer hexagon bevel & facet gradients -->
    <linearGradient id="hex-base" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFB326" />
      <stop offset="35%" stop-color="#F27A18" />
      <stop offset="70%" stop-color="#D9381E" />
      <stop offset="100%" stop-color="#9E1313" />
    </linearGradient>

    <!-- Top facet highlight -->
    <linearGradient id="hex-highlight" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FFF2B8" stop-opacity="0.9" />
      <stop offset="40%" stop-color="#FFA826" stop-opacity="0.2" />
      <stop offset="100%" stop-color="#800000" stop-opacity="0.5" />
    </linearGradient>

    <!-- Inner dark carved channel -->
    <linearGradient id="channel-fill" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#242834" />
      <stop offset="50%" stop-color="#1E202A" />
      <stop offset="100%" stop-color="#15171E" />
    </linearGradient>

    <!-- Rim 3D lighting -->
    <linearGradient id="rim-light" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.4" />
      <stop offset="50%" stop-color="#FF8800" stop-opacity="0.1" />
      <stop offset="100%" stop-color="#400000" stop-opacity="0.6" />
    </linearGradient>

    <!-- Shadow filter for badge -->
    <filter id="badge-shadow" x="-15%" y="-15%" width="130%" height="130%">
      <feDropShadow dx="0" dy="16" stdDeviation="20" flood-color="#000000" flood-opacity="0.45" />
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#000000" flood-opacity="0.3" />
    </filter>

    <filter id="inner-shadow">
      <feComponentTransfer in="SourceAlpha">
        <feFuncA type="linear" slope="0.7"/>
      </feComponentTransfer>
      <feGaussianBlur stdDeviation="3" result="blur"/>
      <feOffset dx="0" dy="3"/>
      <feComposite operator="out" in2="SourceGraphic"/>
      <feColorMatrix type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.8 0"/>
      <feBlend mode="multiply" in2="SourceGraphic"/>
    </filter>
  </defs>

  <g id="yav-hex-badge" filter="url(#badge-shadow)">
    <!-- Base 3D Hexagon Body with Rounded Corners -->
    <path d="M 256 36
             C 272 36 395 106 405 116
             C 415 126 446 220 446 256
             C 446 292 415 386 405 396
             C 395 406 272 476 256 476
             C 240 476 117 406 107 396
             C 97 386 66 292 66 256
             C 66 220 97 126 107 116
             C 117 106 240 36 256 36 Z"
          fill="url(#hex-base)" />

    <!-- 3D Beveled Rim & Lighting -->
    <path d="M 256 36
             C 272 36 395 106 405 116
             C 415 126 446 220 446 256
             C 446 292 415 386 405 396
             C 395 406 272 476 256 476
             C 240 476 117 406 107 396
             C 97 386 66 292 66 256
             C 66 220 97 126 107 116
             C 117 106 240 36 256 36 Z"
          fill="url(#rim-light)" />

    <!-- Facet Division Lines & Sheen -->
    <path d="M 256 36 L 256 256 L 446 256 Z" fill="#FFFFFF" fill-opacity="0.08" />
    <path d="M 256 256 L 405 396 L 256 476 Z" fill="#000000" fill-opacity="0.12" />

    <!-- Inner Carved Channel: "Y" with top-right entry & bottom-left exit -->
    <g id="carved-y-channel">
      <!-- The carved "Y" circuit path -->
      <path d="M 405 116
               L 384 140
               L 298 174
               L 298 238
               L 256 280
               L 236 280
               L 204 312
               L 204 336
               L 242 336
               L 270 308
               L 300 238
               L 300 166
               L 256 166
               L 256 230
               L 228 258
               L 228 166
               L 200 166
               L 200 250
               L 238 288
               L 238 312
               L 204 346
               L 156 346
               L 110 392
               L 126 412
               L 172 366
               L 214 366
               L 258 322
               L 258 296
               L 282 272
               L 320 232
               L 320 160
               L 405 116 Z"
            fill="url(#channel-fill)"
            filter="url(#inner-shadow)" />

      <!-- High-fidelity carved "Y" shape with precision path -->
      <path d="
        M 200 166 L 226 166 L 226 238 L 256 270 L 286 238 L 286 166 L 312 166 L 312 178 L 332 178 L 354 136 L 372 150 L 338 200 L 312 200 L 312 248 L 272 298 L 272 316 L 254 336 L 206 336 L 182 360 L 160 340 L 192 314 L 232 314 L 244 300 L 244 286 L 200 242 Z
      " fill="#1C1E26" />

      <!-- Interactive Developer Terminal Prompt "> _" inside bottom branch -->
      <!-- Greater-than sign ">" -->
      <path d="M 214 322 L 222 326 L 214 330" fill="none" stroke="#F5A623" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
      <!-- Cursor underscore "_" -->
      <path d="M 228 330 L 236 330" fill="none" stroke="#F5A623" stroke-width="2.5" stroke-linecap="round" />
    </g>

    <!-- Inner highlight on carved rim -->
    <path d="M 200 166 L 226 166" stroke="#FFFFFF" stroke-width="1.5" stroke-opacity="0.4" stroke-linecap="round" />
    <path d="M 286 166 L 312 166" stroke="#FFFFFF" stroke-width="1.5" stroke-opacity="0.4" stroke-linecap="round" />
  </g>
</svg>"""

# ---------------------------------------------------------------------------
# 4. Document Logo (yav-document.svg)
# ---------------------------------------------------------------------------
SVG_DOCUMENT = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Document card body gradient -->
    <linearGradient id="doc-card-body" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#464956" />
      <stop offset="60%" stop-color="#383A44" />
      <stop offset="100%" stop-color="#282930" />
    </linearGradient>

    <!-- Bookmark ribbon gold gradient -->
    <linearGradient id="ribbon-gold" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FFDE43" />
      <stop offset="100%" stop-color="#F59E0B" />
    </linearGradient>

    <!-- Folded corner front face -->
    <linearGradient id="fold-front" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFF085" />
      <stop offset="50%" stop-color="#FFCD1E" />
      <stop offset="100%" stop-color="#E69500" />
    </linearGradient>

    <!-- Folded corner shadow underneath -->
    <linearGradient id="fold-shadow" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#14151B" stop-opacity="0.75" />
      <stop offset="100%" stop-color="#000000" stop-opacity="0.0" />
    </linearGradient>

    <!-- Center Emblem Gold -->
    <linearGradient id="emblem-gold" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFDD44" />
      <stop offset="50%" stop-color="#F5A623" />
      <stop offset="100%" stop-color="#D97706" />
    </linearGradient>

    <!-- Drop Shadow Filter for Document Card -->
    <filter id="doc-shadow" x="-15%" y="-15%" width="130%" height="135%">
      <feDropShadow dx="0" dy="18" stdDeviation="22" flood-color="#000000" flood-opacity="0.55" />
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#000000" flood-opacity="0.3" />
    </filter>

    <!-- Emblem subtle shadow -->
    <filter id="emblem-shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="6" stdDeviation="8" flood-color="#000000" flood-opacity="0.4" />
    </filter>
  </defs>

  <g id="yav-document" filter="url(#doc-shadow)">
    <!-- Document Card Body with Fold Cutout -->
    <!-- Card dimensions: x=116, y=72, w=280, h=368, r=24 -->
    <!-- Top right corner is clipped from (320, 72) down to (396, 148) -->
    <path d="M 140 72
             L 320 72
             L 396 148
             L 396 416
             C 396 429.3 385.3 440 372 440
             L 140 440
             C 126.7 440 116 429.3 116 416
             L 116 96
             C 116 82.7 126.7 72 140 72 Z"
          fill="url(#doc-card-body)" />

    <!-- Card Top Edge Highlight Stroke -->
    <path d="M 140 73 L 320 73" stroke="#FFFFFF" stroke-opacity="0.15" stroke-width="1.5" />

    <!-- Fold Shadow (underneath the folded flap) -->
    <path d="M 320 72 L 320 148 L 396 148 Z" fill="url(#fold-shadow)" />

    <!-- Folded Corner Flap (Curved & Luminous) -->
    <path d="M 320 72
             L 320 136
             C 320 142.6 325.4 148 332 148
             L 396 148
             Z"
          fill="url(#fold-front)" />

    <!-- Bookmark Ribbon (Top-Left with Chevron Cut) -->
    <!-- Ribbon: x=148, w=44, y=72, h=76, notch at 134 -->
    <path d="M 148 72
             L 192 72
             L 192 148
             L 170 134
             L 148 148
             Z"
          fill="url(#ribbon-gold)" />

    <!-- Center Geometric "Y" Emblem -->
    <!-- Bold, balanced capital Y -->
    <g id="center-y" filter="url(#emblem-shadow)">
      <path d="M 194 210
               L 230 210
               L 256 264
               L 282 210
               L 318 210
               L 274 286
               L 274 340
               L 238 340
               L 238 286
               Z"
            fill="url(#emblem-gold)" />
      <!-- Subtle internal facet highlight on left stroke of Y -->
      <path d="M 194 210 L 230 210 L 256 264 L 244 264 Z" fill="#FFFFFF" fill-opacity="0.12" />
    </g>
  </g>
</svg>"""


def main():
    files = {
        "yav-monogram-flat.svg": SVG_MONOGRAM_FLAT,
        "yav-monogram-gradient.svg": SVG_MONOGRAM_GRADIENT,
        "yav-hex-badge.svg": SVG_HEX_BADGE,
        "yav-document.svg": SVG_DOCUMENT,
    }
    for filename, content in files.items():
        filepath = os.path.join(SVG_DIR, filename)
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(content.strip() + "\n")
        print(f"Generated {filepath} ({len(content)} bytes)")


if __name__ == "__main__":
    main()
