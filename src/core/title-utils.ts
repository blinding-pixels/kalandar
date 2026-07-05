import { normalizeWhitespace } from './utils.ts';

export const stripReminderLeadPrefix = (input: string): string => {
  let output = normalizeWhitespace(input);
  let previous = '';
  while (output !== previous) {
    previous = output;
    output = normalizeWhitespace(
      output.replace(
        /^(?:remind\s+me\s+to|remind\s+me|don['\u2019]?t\s+forget\s+to|don['\u2019]?t\s+let\s+me\s+forget\s+to)\b[\s,:-]*/i,
        ' ',
      ),
    );
  }
  return output;
};

export const stripTrailingDanglingPreposition = (input: string, enabled = false): string => {
  if (!enabled) {
    return normalizeWhitespace(input);
  }

  let output = normalizeWhitespace(input);
  let previous = '';
  while (output !== previous) {
    previous = output;
    output = normalizeWhitespace(output.replace(/\b(?:for|to|by|with|at|on|in|from)\b[.,;:!?-]*$/i, ' '));
  }
  return output;
};

export const stripLeadingTitleFiller = (input: string): string => {
  let output = normalizeWhitespace(input);
  let previous = '';
  while (output !== previous) {
    previous = output;
    output = normalizeWhitespace(output.replace(/^(?:to(?:\s+the)?|that(?:\s+i)?|about)\b[\s,:-]*/i, ' '));
  }
  return output;
};

export const stripResidualTemporalTokens = (input: string): string => {
  const output = input
    .replace(/\b(?:eod|eow|tmr)\b/gi, ' ')
    .replace(/\bin\s+\d+\s+(?:day|days|week|weeks|month|months)\b/gi, ' ')
    .replace(/\bevery\s+other\s+(?:day|week)\b/gi, ' ')
    .replace(/\bevery\s+\d+\s+(?:days|weeks|months)\b/gi, ' ')
    .replace(/\b(?:next|this|last)\s+(?:month|week)\b/gi, ' ')
    .replace(
      /\b(?:next|this|last)\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b/gi,
      ' ',
    )
    .replace(/\bend of (?:the )?month\b/gi, ' ')
    .replace(/\bend of (?:the )?week\b/gi, ' ')
    .replace(/\bend of (?:the )?day\b/gi, ' ')
    .replace(/\b(?:today|tomorrow|tonight|yesterday)\b/gi, ' ');

  return normalizeWhitespace(output);
};
