export type ConvertRequestMessage = {
    input: string
}
export type ConvertResponseMessage = {
    output: string
} | {
    error: string
}
