<div class="bg">
  <svg class="bg-noise" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <filter id="n"><feTurbulence type="fractalNoise" baseFrequency="0.7" numOctaves="4" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/></filter>
    <rect width="100%" height="100%" filter="url(#n)"/>
  </svg>
  <div class="bg-glow"></div>
</div>

<style>
  .bg { position: fixed; inset: 0; z-index: -10; background: #000; }
  .bg-noise { position: absolute; inset: 0; width: 100%; height: 100%; opacity: 0.025; }
  .bg-glow {
    position: absolute; top: -80px; left: 50%; transform: translateX(-50%);
    width: 500px; height: 280px;
    background: radial-gradient(ellipse, rgba(10,132,255,0.15) 0%, transparent 70%);
    border-radius: 50%; filter: blur(1px); pointer-events: none;
  }
</style>
