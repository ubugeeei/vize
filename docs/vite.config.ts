import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig } from "vite";
import { oxContent, defineTheme, defaultTheme } from "@ox-content/vite-plugin";

const themeDir = resolve(import.meta.dirname, "theme");
const themeCss = readFileSync(resolve(themeDir, "style.css"), "utf-8");

const shaderDir = resolve(themeDir, "shaders");
const vertSrc = readFileSync(resolve(shaderDir, "marble.vert"), "utf-8");
const fragSrc = readFileSync(resolve(shaderDir, "marble.frag"), "utf-8");
const themeJs = readFileSync(resolve(themeDir, "marble.js"), "utf-8")
  .replace("__VERT_SRC__", vertSrc.replace(/`/g, "\\`"))
  .replace("__FRAG_SRC__", fragSrc.replace(/`/g, "\\`"));

export default defineConfig({
  plugins: [
    oxContent({
      srcDir: "content",
      outDir: "dist",

      ogImage: true,
      ogImageOptions: {
        template: resolve(themeDir, "og.vue"),
        vuePlugin: "vizejs",
        width: 1200,
        height: 630,
        cache: true,
      },

      ssg: {
        siteName: "Vize",
        siteUrl: "https://vizejs.dev",
        generateOgImage: true,
        theme: defineTheme({
          extends: defaultTheme,

          colors: {
            primary: "#121212",
            primaryHover: "#333333",
            background: "#e6e2d6",
            backgroundAlt: "#dedad0",
            text: "#121212",
            textMuted: "#5a5750",
            border: "#ccc8bc",
            codeBackground: "#1a1a1a",
            codeText: "#e8e4dc",
          },

          darkColors: {
            primary: "#e8e4dc",
            primaryHover: "#ffffff",
            background: "#161616",
            backgroundAlt: "#1c1c1c",
            text: "#e8e4dc",
            textMuted: "#8a8780",
            border: "#1e1e1e",
            codeBackground: "#0f0f0f",
            codeText: "#e8e4dc",
          },

          fonts: {
            sans: '"Helvetica Neue", Helvetica, Arial, system-ui, sans-serif',
            mono: '"JetBrains Mono", ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace',
          },

          header: {
            logo: "/logo.svg",
            logoWidth: 32,
            logoHeight: 32,
          },

          footer: {
            message:
              'Released under the <a href="https://opensource.org/licenses/MIT">MIT License</a>.',
            copyright: `Copyright &copy; 2024-${new Date().getFullYear()} ubugeeei`,
          },

          socialLinks: {
            github: "https://github.com/ubugeeei/vize",
          },

          embed: {
            head: [
              '<link rel="preconnect" href="https://fonts.googleapis.com">',
              '<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>',
              '<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">',
              '<script src="https://cdn.jsdelivr.net/npm/three@0.160.0/build/three.min.js"><\/script>',
              "<script>if(!localStorage.getItem('theme')){localStorage.setItem('theme','light')}<\/script>",
            ].join("\n"),
            headerAfter:
              '<canvas id="marble-canvas" style="position:fixed;top:0;left:0;width:100%;height:100%;z-index:-1;pointer-events:none;"></canvas>',
          },

          css: themeCss,
          js: themeJs,
        }),
      },

      highlight: true,
      highlightTheme: "vitesse-dark",
      mermaid: true,
    }),
  ],

  server: {
    port: 4200,
  },
  preview: {
    port: 4200,
  },
  build: {
    outDir: "dist",
  },
});
