import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { WaftIcon } from '@/components/waft-icon';
import { appName, docsRoute, repositoryUrl } from './shared';

/** The header: the icon and name, Docs, docs.rs menu, GitHub and Fumadocs' search. */
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="flex items-center gap-2 font-semibold">
          <WaftIcon size={24} />
          {appName}
        </span>
      ),
    },
    links: [
      { text: 'Docs', url: docsRoute, active: 'nested-url' },
      {
        type: 'menu',
        text: 'docs.rs',
        items: [
          {
            text: 'waft',
            description: 'Core widgets, layout, handles, and #[waft::main]',
            url: 'https://docs.rs/waft/latest/waft/',
            external: true,
          },
          {
            text: 'waft-winui',
            description: 'WinUI 3 / Fluent controls for desktop and web',
            url: 'https://docs.rs/waft-winui/latest/waft_winui/',
            external: true,
          },
          {
            text: 'waft-cupertino',
            description: 'iOS / Cupertino controls, sheets, and spring scroll',
            url: 'https://docs.rs/waft-cupertino/latest/waft_cupertino/',
            external: true,
          },
          {
            text: 'waft-material',
            description: 'Material Design 3 controls and tonal color schemes',
            url: 'https://docs.rs/waft-material/latest/waft_material/',
            external: true,
          },
          {
            text: 'waft-embedder',
            description: 'Platform, compositor, media, and host embedder traits',
            url: 'https://docs.rs/waft-embedder/latest/waft_embedder/',
            external: true,
          },
          {
            text: 'All crates reference',
            description: 'Complete index of published Waft crates on docs.rs',
            url: `${docsRoute}/reference`,
          },
        ],
      },
    ],
    githubUrl: repositoryUrl,
    themeSwitch: { enabled: false },
  };
}
