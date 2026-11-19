export const appName = 'Waft';
/** The first section's statement, in two lines. */
export const headline = ['Solid, fluent apps in Rust.', 'Desktop, mobile, web, and WASI.'] as const;
export const subtitle =
  "Flutter's widget model in pure Rust, rendered by valo and wgpu with mature WinUI, Cupertino, and Material kits.";
export const tagline = headline.join(' ');
export const description = subtitle;

export function siteOrigin(): string {
  return process.env.NEXT_PUBLIC_SITE_URL ?? 'https://waft.rs';
}

export const docsRoute = '/docs';

export const releaseVersion = '0.8.2';
export const repositoryUrl = 'https://github.com/waos-dev/waft';
export const cratesIoUrl = 'https://crates.io/crates/waft';
export const docsRsUrl = 'https://docs.rs/waft';
export const winuiGalleryUrl = 'https://winui-gallery.inset.rs';
export const cupertinoGalleryUrl = 'https://cupertino-gallery.inset.rs';
export const edgedUrl = 'https://edged2.app';
export const waosUrl = 'https://waos.app';

/** One line pointing an agent at its playbook, optionally tailored to a specific app idea. */
export function agentPrompt(idea?: string, origin = siteOrigin()): string {
  const target = idea ? `a ${idea}` : 'a cross-platform app';
  return `Read ${origin}/agents.md and follow it to build ${target} with Waft.`;
}

export const githubUrl = repositoryUrl;
export const galleryUrls = { winui: winuiGalleryUrl, cupertino: cupertinoGalleryUrl } as const;
