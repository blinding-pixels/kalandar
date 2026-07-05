import type { ContactSuggestion } from './types.ts';

const CONTACT_BOUNDARY_REGEX =
  /\b(?:every|starting|beginning|unless|except|but skip|skip|at|on|from|for|until|through|ending|tomorrow|next)\b/i;
const WITH_PATTERN = /\b(?:with|w\/)\s+(.+)/i;
const SPLIT_PATTERN = /\s*(?:,| and | & )\s*/i;

export const extractCalendarContactSuggestions = (title: string): ContactSuggestion[] => {
  const match = WITH_PATTERN.exec(title);
  if (!match) {
    return [];
  }

  let candidateText = (match[1] || '').trim().replace(/[ ,;.]+$/g, '');
  const boundary = CONTACT_BOUNDARY_REGEX.exec(candidateText);
  if (boundary) {
    candidateText = candidateText.slice(0, boundary.index).trim().replace(/[ ,;.]+$/g, '');
  }

  if (!candidateText) {
    return [];
  }

  const suggestions: ContactSuggestion[] = [];
  const seen = new Set<string>();
  for (const segment of candidateText.split(SPLIT_PATTERN)) {
    const cleaned = segment.trim().replace(/[ ,;.]+$/g, '');
    const lowered = cleaned.toLowerCase();
    if (cleaned.length < 2 || lowered.includes('@') || seen.has(lowered)) {
      continue;
    }
    seen.add(lowered);
    suggestions.push({ text: cleaned, source: 'title_with_phrase' });
  }

  return suggestions;
};
