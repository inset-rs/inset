import { getPageMarkdownUrl, source } from '@/lib/source';
import {
  DocsBody,
  DocsDescription,
  DocsPage,
  DocsTitle,
  MarkdownCopyButton,
  ViewOptionsPopover,
} from 'fumadocs-ui/layouts/docs/page';
import { notFound } from 'next/navigation';
import { getMDXComponents } from '@/components/mdx';
import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { createRelativeLink } from 'fumadocs-ui/mdx';
import Link from 'next/link';
import { topicIcon } from '@/lib/topic-icons';

export default async function Page(props: PageProps<'/docs/[[...slug]]'>) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;
  const markdownUrl = getPageMarkdownUrl(page).url;

  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <div className="flex flex-wrap items-start justify-between gap-x-6 gap-y-3">
        <DocsTitle className="text-4xl font-semibold tracking-tight">{page.data.title}</DocsTitle>
        <div className="flex flex-row items-center gap-2 pt-1.5 opacity-70 transition-opacity hover:opacity-100">
          <MarkdownCopyButton markdownUrl={markdownUrl} />
          <ViewOptionsPopover markdownUrl={markdownUrl} />
        </div>
      </div>
      {page.data.description ? (
        <DocsDescription className="mb-2 text-xl">{page.data.description}</DocsDescription>
      ) : null}
      <DocsBody>
        <MDX components={getMDXComponents({ a: createRelativeLink(source, page) })} />
        <PagesWithin url={page.url} />
      </DocsBody>
    </DocsPage>
  );
}

/** The pages under the one at `url`, where it opens a section: each with its sign, name and one line, grouped under the sidebar's headings. */
function PagesWithin({ url }: { url: string }) {
  if (url.endsWith('/reference') || url.includes('/reference/')) return null;
  const groups = pageGroups(url);
  if (groups.length === 0) return null;

  return (
    <>
      {groups.map((group, index) => (
        <section key={index}>
          {group.heading ? <h2>{group.heading}</h2> : null}
          <ul className="not-prose mt-2 grid gap-4 sm:grid-cols-2">
            {group.pages.map((page) => (
              <PageCard key={page.url} page={page} />
            ))}
          </ul>
        </section>
      ))}
    </>
  );
}

type DocsPage = ReturnType<typeof source.getPages>[number];

function PageCard({ page }: { page: DocsPage }) {
  const Icon = topicIcon(page.url);
  return (
    <li>
      <Link
        href={page.url}
        className="flex h-full items-start gap-4 rounded-2xl border bg-fd-card p-5 transition-colors hover:bg-fd-accent"
      >
        <span className="flex size-10 shrink-0 items-center justify-center rounded-xl border bg-fd-background">
          <Icon aria-hidden className="size-5" strokeWidth={1.75} />
        </span>
        <span className="flex flex-col gap-1">
          <span className="text-lg font-semibold">{page.data.title}</span>
          <span className="text-fd-muted-foreground">{page.data.description}</span>
        </span>
      </Link>
    </li>
  );
}

/** The pages of the section `url` opens, in the sidebar's order, a group under each of its headings. */
function pageGroups(url: string): { heading?: ReactNode; pages: DocsPage[] }[] {
  const folder = source.getPageTree().children.find((node) => node.type === 'folder' && node.index?.url === url);
  if (folder?.type !== 'folder') return [];
  const groups: { heading?: ReactNode; pages: DocsPage[] }[] = [];
  for (const node of folder.children) {
    if (node.type === 'separator') {
      groups.push({ heading: node.name, pages: [] });
    } else if (node.type === 'page') {
      const page = source.getPages().find((candidate) => candidate.url === node.url);
      if (!page) continue;
      if (groups.length === 0) groups.push({ pages: [] });
      groups[groups.length - 1].pages.push(page);
    }
  }
  return groups.filter((group) => group.pages.length > 0);
}

export async function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: PageProps<'/docs/[[...slug]]'>): Promise<Metadata> {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  return {
    title: page.data.title,
    description: page.data.description,
    alternates: {
      canonical: page.url,
      types: { 'text/markdown': getPageMarkdownUrl(page).url },
    },
    openGraph: {
      type: 'article',
      title: page.data.title,
      description: page.data.description,
    },
  };
}
