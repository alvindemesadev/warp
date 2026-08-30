<div class="bg">
  <svg class="bg-noise" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <filter id="n"
      ><feTurbulence
        type="fractalNoise"
        baseFrequency="0.7"
        numOctaves="4"
        stitchTiles="stitch"
      /><feColorMatrix type="saturate" values="0" /></filter
    >
    <rect width="100%" height="100%" filter="url(#n)" />
  </svg>
  <div class="bg-glow"></div>
</div>

<style>
  .bg {
    position: fixed;
    inset: 0;
    z-index: -10;
    background: var(--surface-0);
    transition: background 0.25s ease;
  }
  .bg-noise {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: var(--noise-opacity, 0.025);
    pointer-events: none;
  }
  .bg-glow {
    position: absolute;
    top: -80px;
    left: 50%;
    transform: translateX(-50%);
    width: 500px;
    height: 280px;
    background: var(--glow-bg);
    border-radius: 50%;
    filter: blur(2px);
    pointer-events: none;
    transition: background 0.25s ease;
  }
</style>
