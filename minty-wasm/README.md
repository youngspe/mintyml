[MinTyML](https://youngspe.github.io/mintyml/)
is an alternative HTML syntax intended for writing documents.

## Usage

```ts
import { MintymlConverter } from 'mintyml'

const converter = new MintymlConverter({
    xml: false, // default: false
    indent: 2, // default: null
    completePage: false, // default: false
})

converter.convert(`\
article {
    h1> Title

    Hello, world!
}
`)
```
