import adapter from "@sveltejs/adapter-static";

const basePath = process.env.BASE_PATH || "";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    adapter: adapter({
      pages: "build",
      assets: "build",
      fallback: "index.html",
      precompress: false,
      strict: true,
    }),
    alias: {
      $lib: "src/lib",
    },
    paths: {
      base: basePath,
    },
  },
};

export default config;
