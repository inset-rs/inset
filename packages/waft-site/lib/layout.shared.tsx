import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { WaftIcon } from '@/components/waft-icon';
import { appName, docsRoute, repositoryUrl } from './shared';

/** The header: the icon and name, Docs, GitHub and Fumadocs' search, in colors that follow the system's. */
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
    links: [{ text: 'Docs', url: docsRoute, active: 'nested-url' }],
    githubUrl: repositoryUrl,
    themeSwitch: { enabled: false },
  };
}
