import {
  AppWindow,
  BookOpen,
  Boxes,
  Compass,
  Cpu,
  Database,
  FlaskConical,
  Keyboard,
  Layers,
  LayoutGrid,
  Package,
  Palette,
  TextCursorInput,
  type LucideIcon,
} from 'lucide-react';
import { docsRoute } from './shared';

const icons: Record<string, LucideIcon> = {
  [`${docsRoute}/start`]: Compass,
  [`${docsRoute}/architecture`]: Layers,
  [`${docsRoute}/state`]: Database,
  [`${docsRoute}/handles`]: Boxes,
  [`${docsRoute}/layout`]: LayoutGrid,
  [`${docsRoute}/text-input`]: TextCursorInput,
  [`${docsRoute}/kits`]: Palette,
  [`${docsRoute}/pages`]: AppWindow,
  [`${docsRoute}/shortcuts`]: Keyboard,
  [`${docsRoute}/testing`]: FlaskConical,
  [`${docsRoute}/cli`]: Package,
  [`${docsRoute}/embedder`]: Cpu,
  [`${docsRoute}/reference`]: BookOpen,
};

export function topicIcon(url: string): LucideIcon {
  return icons[url] ?? BookOpen;
}
