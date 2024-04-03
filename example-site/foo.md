title = "Article FOO!"
date = "1/1/1970"
page = "article"
max_depth = 2
summary = """\
Demonstrate a bit of Verin
"""
/~

## On the TOC

```json
{
    "highlighted_json": true,
}
```

```devicetree
/dts-v1/;

/ {
    soc {
        flash_controller: flash-controller@4001e000 {
            reg = <0x4001e000 0x1000>;
            flash0: flash@0 {
                label = "SOC_FLASH";
                erase-block = <4096>;
            };
        };
    };
};
```

### Not on the TOC

This is a section that is not present in the TOC as it is too nested
