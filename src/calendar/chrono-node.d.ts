declare module 'chrono-node' {
  export interface ParsedComponents {
    get(name: string): number | null;
    isCertain(name: string): boolean;
    date(): Date;
  }

  export interface ParsedResult {
    index: number;
    text: string;
    start: ParsedComponents;
    end?: ParsedComponents;
  }

  export function parse(
    text: string,
    referenceDate?: Date,
    options?: {
      forwardDate?: boolean;
    },
  ): ParsedResult[];
}
