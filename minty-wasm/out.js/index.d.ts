/** Represents a failed MinTyML operation. */
export interface MintymlError {
    /** Message describing the error. */
    message: string;
    /** The specific syntax errors that caused the failure. */
    syntaxErrors?: MintymlSyntaxError[];
}
/** Base type for any MinTyML syntax error. */
export interface MintymlBaseSyntaxError {
    /** Message describing the syntax error. */
    message: string;
    /** The source text causing the syntax error. */
    actual: string;
    /** The location within the source string where the error begins. */
    start: number;
    /** The location within the source string where the error ends. */
    end: number;
}
/** Represents a scenario where the source could not be parsed into a tree. */
export interface MintymlParsingError extends MintymlBaseSyntaxError {
    /** A list of strings describing what was expected. */
    expected: string[];
}
/** Describes a syntax error that caused a failed MinTyML operation. */
export type MintymlSyntaxError = MintymlBaseSyntaxError | MintymlParsingError;
export interface MintymlConverterOptions {
    /**
     * If true, produce XHTML5 rather than HTML.
     * @default false
     */
    xml: boolean;
    /**
     * If specified, the converted HTML will be pretty-printed with the given number
     * of spaces in each indentation level.
     * If null, the converted HTML will not be pretty-printed.
     *
     * @default null
     */
    indent: number | null;
    /**
     * If true, the output will be a complete HTML page with an `<html>` tag containing
     * a `<body>` and `<head>` tag.
     *
     * Otherwise, keep the input's element structure intact.
     *
     * @default false
     */
    completePage: boolean;
}
/** Converts MinTyML source to HTML. */
export declare class MintymlConverter implements MintymlConverterOptions {
    xml: boolean;
    indent: number | null;
    completePage: boolean;
    constructor(options?: Partial<MintymlConverterOptions>);
    /** Converts the given MinTyML string to HTML. */
    convert(src: string): Promise<string>;
}
