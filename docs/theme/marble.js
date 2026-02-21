// Marble Veining — Three.js Interactive Shader
;(() => {
  if (typeof THREE === 'undefined') {
    console.warn('[marble] Three.js not loaded');
    return;
  }

  const canvas = document.getElementById('marble-canvas');
  if (!canvas) return;

  // --- Renderer ---
  const renderer = new THREE.WebGLRenderer({
    canvas,
    alpha: true,
    premultipliedAlpha: false,
    antialias: false,
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));

  // --- Scene & Camera (fullscreen quad) ---
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);

  // --- Uniforms ---
  const uniforms = {
    u_resolution: { value: new THREE.Vector2() },
    u_time: { value: 0.0 },
    u_scroll: { value: 0.0 },
    u_dark: { value: 0.0 },
    u_mouse: { value: new THREE.Vector2(0.5, 0.5) },
    u_mouseVel: { value: 0.0 },
  };

  const material = new THREE.ShaderMaterial({
    vertexShader: `__VERT_SRC__`,
    fragmentShader: `__FRAG_SRC__`,
    uniforms,
    transparent: true,
    depthWrite: false,
  });

  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), material);
  scene.add(mesh);

  // --- Mouse tracking with smooth lerp ---
  let mouseX = 0.5, mouseY = 0.5;
  let targetX = 0.5, targetY = 0.5;
  let mouseVel = 0.0;
  let prevMX = 0.5, prevMY = 0.5;

  document.addEventListener('mousemove', (e) => {
    targetX = e.clientX / window.innerWidth;
    targetY = 1.0 - e.clientY / window.innerHeight;
  }, { passive: true });

  document.addEventListener('touchmove', (e) => {
    if (e.touches.length > 0) {
      targetX = e.touches[0].clientX / window.innerWidth;
      targetY = 1.0 - e.touches[0].clientY / window.innerHeight;
    }
  }, { passive: true });

  // --- Scroll ---
  let scrollY = 0;
  const header = document.querySelector('.header');
  window.addEventListener('scroll', () => {
    scrollY = window.scrollY;
    // Show header after scrolling past hero
    if (header && document.body.classList.contains('entry-page')) {
      if (scrollY > 100) {
        header.classList.add('header-visible');
      } else {
        header.classList.remove('header-visible');
      }
    }
  }, { passive: true });

  // --- Dark mode detection ---
  const isDark = () => {
    const theme = document.documentElement.getAttribute('data-theme');
    if (theme === 'dark') return true;
    if (theme === 'light') return false;
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  };

  // --- Resize ---
  const resize = () => {
    renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
    uniforms.u_resolution.value.set(canvas.width, canvas.height);
  };
  window.addEventListener('resize', resize);
  resize();

  // --- Animation loop ---
  const clock = new THREE.Clock();

  const render = () => {
    requestAnimationFrame(render);

    const dt = clock.getDelta();
    uniforms.u_time.value = clock.getElapsedTime();
    uniforms.u_scroll.value = scrollY;
    uniforms.u_dark.value = isDark() ? 1.0 : 0.0;

    // Smooth mouse lerp
    mouseX += (targetX - mouseX) * 0.05;
    mouseY += (targetY - mouseY) * 0.05;

    // Mouse velocity
    const dx = mouseX - prevMX;
    const dy = mouseY - prevMY;
    const speed = Math.sqrt(dx * dx + dy * dy) / Math.max(dt, 0.001);
    mouseVel += (speed - mouseVel) * 0.1;
    prevMX = mouseX;
    prevMY = mouseY;

    uniforms.u_mouse.value.set(mouseX, mouseY);
    uniforms.u_mouseVel.value = mouseVel;

    renderer.render(scene, camera);
  };

  render();

  // Theme change observer
  new MutationObserver(() => {}).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  });
})();
