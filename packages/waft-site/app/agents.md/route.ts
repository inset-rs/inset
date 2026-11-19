import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

export const revalidate = false;

/** The agent's playbook (decision 7), the page the home page's prompt points an agent at. */
export async function GET() {
  const playbook = await readFile(join(process.cwd(), 'content/agents.md'), 'utf8');
  return new Response(playbook, {
    headers: { 'Content-Type': 'text/markdown; charset=utf-8' },
  });
}
