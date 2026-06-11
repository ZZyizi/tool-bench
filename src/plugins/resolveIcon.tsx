import { Box, type LucideIcon } from 'lucide-react';
import * as Lucide from 'lucide-react';
import type { IconRef } from './types';

// plugin.json 里 icon 是字符串（如 "Plug"），但在 Plugin.manifest 里也可以
// 直接是 ComponentType（index.tsx 里手动 import 赋值）。
// UI 层统一通过 resolveIcon 拿 LucideIcon。
export function resolveIcon(icon: IconRef | undefined): LucideIcon {
  if (!icon) return Box;
  if (typeof icon === 'function') return icon as LucideIcon;
  const fromLib = (Lucide as unknown as Record<string, LucideIcon | undefined>)[icon];
  if (fromLib) return fromLib;
  console.warn(`[plugins] icon "${icon}" not found in lucide-react, falling back to Box`);
  return Box;
}
