precision mediump float;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_scroll;
uniform float u_dark;
uniform vec2 u_mouse;
uniform float u_mouseVel;

varying vec2 vUv;

// --- Noise ---
vec3 mod289(vec3 x) { return x - floor(x * (1.0/289.0)) * 289.0; }
vec2 mod289(vec2 x) { return x - floor(x * (1.0/289.0)) * 289.0; }
vec3 permute(vec3 x) { return mod289(((x*34.0)+1.0)*x); }

float snoise(vec2 v) {
  const vec4 C = vec4(0.211324865405187, 0.366025403784439,
                       -0.577350269189626, 0.024390243902439);
  vec2 i = floor(v + dot(v, C.yy));
  vec2 x0 = v - i + dot(i, C.xx);
  vec2 i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
  vec4 x12 = x0.xyxy + C.xxzz;
  x12.xy -= i1;
  i = mod289(i);
  vec3 p = permute(permute(i.y + vec3(0.0, i1.y, 1.0)) + i.x + vec3(0.0, i1.x, 1.0));
  vec3 m = max(0.5 - vec3(dot(x0,x0), dot(x12.xy,x12.xy), dot(x12.zw,x12.zw)), 0.0);
  m = m*m; m = m*m;
  vec3 x = 2.0 * fract(p * C.www) - 1.0;
  vec3 h = abs(x) - 0.5;
  vec3 ox = floor(x + 0.5);
  vec3 a0 = x - ox;
  m *= 1.79284291400159 - 0.85373472095314 * (a0*a0 + h*h);
  vec3 g;
  g.x = a0.x * x0.x + h.x * x0.y;
  g.yz = a0.yz * x12.xz + h.yz * x12.yw;
  return 130.0 * dot(m, g);
}

// Flowing noise for vein displacement
float flowNoise(vec2 p, float t) {
  return snoise(p + vec2(t * 0.1, 0.0))
       + 0.5 * snoise(p * 2.1 + vec2(0.0, t * 0.08))
       + 0.25 * snoise(p * 4.3 + vec2(t * 0.06, t * 0.04));
}

// Single marble vein line
float veinLine(vec2 p, float direction, float noiseScale, float noiseAmp, float width, float t) {
  float coord = p.x * cos(direction) + p.y * sin(direction);
  float displacement = flowNoise(p * noiseScale + vec2(t), t) * noiseAmp;
  float line = sin(coord * 3.14159 + displacement);
  return smoothstep(width, 0.0, abs(line)) * smoothstep(0.0, width * 3.0, abs(line) + 0.001);
}

void main() {
  vec2 uv = vUv;
  float aspect = u_resolution.x / u_resolution.y;
  vec2 p = vec2(uv.x * aspect, uv.y);

  float t = u_time * 0.008;
  float scroll = u_scroll * 0.0001;

  // --- Mouse influence ---
  vec2 mp = vec2(u_mouse.x * aspect, u_mouse.y);
  float dist = length(p - mp);
  float influence = smoothstep(1.0, 0.0, dist) * 0.15;
  float velInfluence = min(u_mouseVel * 0.1, 0.2);
  vec2 dir = normalize(p - mp + 0.001);
  vec2 wp = p + dir * influence * (0.08 + velInfluence * 0.04);

  // Apply scroll offset
  wp.y += scroll;

  // --- Main diagonal veins (broad, straight-ish) ---
  float v1 = veinLine(wp, 0.55, 0.5, 1.2, 0.22, t);
  float v2 = veinLine(wp + vec2(3.0, 1.5), 0.65, 0.6, 1.4, 0.18, t * 0.9);

  // --- Secondary veins (thinner, slight angle variation) ---
  float v3 = veinLine(wp + vec2(7.0, -2.0), 0.4, 0.7, 1.0, 0.12, t * 1.1);
  float v4 = veinLine(wp + vec2(-5.0, 4.0), 0.75, 0.65, 1.1, 0.10, t * 0.85);

  // --- Fine hairline veins ---
  float v5 = veinLine(wp + vec2(11.0, 6.0), 0.5, 1.0, 0.8, 0.05, t * 1.2);
  float v6 = veinLine(wp + vec2(-3.0, 9.0), 0.7, 0.9, 0.9, 0.05, t * 0.7);

  // --- Combine ---
  float pattern = v1 * 1.0 + v2 * 0.85
                + v3 * 0.6 + v4 * 0.55
                + v5 * 0.35 + v6 * 0.3;

  // Mouse proximity glow
  float mouseGlow = smoothstep(0.5, 0.0, dist) * (0.08 + velInfluence * 0.12);
  pattern += mouseGlow;

  // Subtle vignette
  float vig = smoothstep(0.0, 0.3, uv.x) * smoothstep(1.0, 0.7, uv.x);
  vig *= smoothstep(0.0, 0.25, uv.y) * smoothstep(1.0, 0.75, uv.y);
  vig = mix(0.4, 1.0, vig);

  float intensity = pattern * vig;

  if (u_dark > 0.5) {
    // Dark: bright ivory vein streaks
    float di = intensity * 2.5;
    vec3 col = vec3(0.95, 0.92, 0.85) * di;
    gl_FragColor = vec4(col, min(di, 1.0));
  } else {
    // Light: warm stone vein streaks
    vec3 col = vec3(0.18, 0.15, 0.10) * intensity;
    gl_FragColor = vec4(col, intensity);
  }
}
