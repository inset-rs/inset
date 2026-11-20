import { loader } from 'fumadocs-core/source';
import type * as PageTree from 'fumadocs-core/page-tree';
import { defineDocs } from 'fumadocs-mdx/macro';
import { metaSchema, pageSchema } from 'fumadocs-core/source/schema';
import { docsRoute, siteOrigin } from './shared';

const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    schema: pageSchema,
    postprocess: {
      includeProcessedMarkdown: true,
    },
  },
  meta: {
    schema: metaSchema,
  },
});

export const source = loader({
  baseUrl: docsRoute,
  source: docs.toFumadocsSource(),
});

type Page = (typeof source)['$inferPage'];

export function getPageMarkdownUrl(page: Page) {
  return { url: `${page.url}.md` };
}

export async function getLLMText(page: Page) {
  const processed = await page.data.getText('processed');

  return `# ${page.data.title} (${siteOrigin()}${page.url})

${processed}`;
}

/** All top-level docs pages in the sidebar's order. */
export function docTopics(): PageTree.Item[] {
  return source
    .getPageTree()
    .children.filter((node): node is PageTree.Item => node.type === 'page');
}
