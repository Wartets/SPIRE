import sveltePreprocess from "svelte-preprocess";

const basePath = process.env.BASE_PATH || "";
const staticAdapterConfig = {
  pages: "build",
  assets: "build",
  fallback: "index.html",
  precompress: false,
  strict: true,
};

async function resolveAdapter() {
  try {
    const { default: adapterStatic } = await import("@sveltejs/adapter-static");
    return adapterStatic(staticAdapterConfig);
  } catch {
    return {
      name: "noop-adapter",
      adapt: async () => {},
    };
  }
}

const adapter = await resolveAdapter();

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: sveltePreprocess(),
  kit: {
    adapter,
    alias: {
      $lib: "src/lib",
    },
    paths: {
      base: basePath,
    },
  },
};

export default config;
