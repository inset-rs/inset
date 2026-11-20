import { RootProvider } from 'fumadocs-ui/provider/next';
import type { Metadata } from 'next';
import './global.css';
import { appName, description, siteOrigin } from '@/lib/shared';

const origin = siteOrigin();

export const metadata: Metadata = {
  metadataBase: new URL(origin),
  title: { default: appName, template: `%s — ${appName}` },
  description,
  applicationName: appName,
  alternates: { canonical: '/' },
  openGraph: {
    type: 'website',
    locale: 'en_US',
    url: origin,
    siteName: appName,
    title: appName,
    description,
  },
  twitter: {
    card: 'summary',
    title: appName,
    description,
  },
  robots: { index: true, follow: true },
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    // `suppressHydrationWarning` because the provider writes the theme to `documentElement`
    // before React hydrates.
    <html lang="en" suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
