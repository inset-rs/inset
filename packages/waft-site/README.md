# waft-site

Waft's website and documentation, `waft.rs`: a Next.js + Fumadocs site with a light-first editorial homepage (interactive UI Kit Studio across WinUI, Cupertino, and Material 3; 4-layer crate & `waft-embedder` architecture explorer; desktop & mobile parity specification; headless `AppCell` testing & `cargo waft` CLI showcase; and coding-agent prompt card), 13 documentation guides under `/docs/*`, `/agents.md`, `/llms.txt`, `/llms-full.txt`, and per-page `.md` endpoints.

```sh
npm install
npm run dev        # at http://localhost:3000
npm run build      # production build in .next
npm start          # serves that build
npm run check      # next typegen && tsc --noEmit
npm test           # Playwright & Markdown/LLM route smoke suite against the production build
```

## On Vercel

The site deploys to Vercel (`xutys-projects/waft`) with Root Directory set to `packages/waft-site` and domain `waft.rs`.
