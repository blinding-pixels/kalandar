const DIRECT_REPLACEMENTS: Array<[RegExp, string]> = [
  [/\btmrw\b/gi, 'tomorrow'],
  [/\btmr\b/gi, 'tomorrow'],
  [/\btomorow\b/gi, 'tomorrow'],
  [/\btommorow\b/gi, 'tomorrow'],
  [/\btmorow\b/gi, 'tomorrow'],
  [/\btonite\b/gi, 'tonight'],
  [/\b2\s*hrs?\b/gi, '2 hours'],
  [/\b(\d+)\s*hrs?\b/gi, '$1 hours'],
  [/\b(\d+)\s*mins?\b/gi, '$1 minutes'],
  [/\b(\d+)\s*min\b/gi, '$1 minutes'],
  [/\b(\d+)\s*secs?\b/gi, '$1 seconds'],
];

const WEEKDAY_SHORTHAND: Array<[RegExp, string]> = [
  [/\bmon\b/gi, 'monday'],
  [/\btue\b/gi, 'tuesday'],
  [/\btues\b/gi, 'tuesday'],
  [/\bwed\b/gi, 'wednesday'],
  [/\bthu\b/gi, 'thursday'],
  [/\bthur\b/gi, 'thursday'],
  [/\bthurs\b/gi, 'thursday'],
  [/\bfri\b/gi, 'friday'],
  [/\bsat\b/gi, 'saturday'],
  [/\bsun\b/gi, 'sunday'],
];

const TIME_OF_DAY_SHORTHAND: Array<[RegExp, string]> = [
  [/\btom(?:orrow)?\s+night\b/gi, 'tomorrow night'],
  [/\btom(?:orrow)?\s+morn(?:ing)?\b/gi, 'tomorrow morning'],
  [/\bfri\s+eve\b/gi, 'friday evening'],
  [/\bfri\s+morn\b/gi, 'friday morning'],
];

export const normalizeShadowText = (input: string): string => {
  let output = input || '';

  for (const [pattern, replacement] of DIRECT_REPLACEMENTS) {
    output = output.replace(pattern, replacement);
  }

  for (const [pattern, replacement] of TIME_OF_DAY_SHORTHAND) {
    output = output.replace(pattern, replacement);
  }

  for (const [pattern, replacement] of WEEKDAY_SHORTHAND) {
    output = output.replace(pattern, replacement);
  }

  output = output
    .replace(/\b(\d{1,2})\s*-\s*(\d{1,2})(?=\b)/g, '$1 to $2')
    .replace(/[–—]/g, '-')
    .replace(/\s+/g, ' ')
    .trim();

  return output;
};
