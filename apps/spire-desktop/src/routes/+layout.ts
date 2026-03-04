// SPA mode: disable server-side rendering, enable client-side prerendering.
// Tauri IPC APIs are browser-only; SSR would fail on the server.
export const ssr = false;
export const prerender = true;
