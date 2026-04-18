# Runner file directives

## Write stores the following fenced block in the test home directory

The `:write` directive should write the next fenced block into the sandbox so later commands can
read it.

    :bins cat python3

    :write nested/hello.txt

```text
hello
from write
```

    :$ cat nested/hello.txt

## Copy imports a manifest-relative file into the test home directory

The `:copy` directive should make it easy to bring a repo fixture into the sandbox without shelling
out.

    :copy scripts/tmcap nested/tmcap

    :$ python3 -c "print(open('nested/tmcap').readline().strip())"

---
vim: set ft=markdown:
