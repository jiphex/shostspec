# shostspec

This is a small utility for converting Slurm (and other batch system/etc) host
expressions (e.g host[120-150,999]) into their composite individual hosts. This
is often required when using information from e.g Slurm (such as the output from
`sinfo`) to do administrative things (like powering on a number of hosts using
`ipmitool`).

## Usage

```shell
[you@box] $ shostspec host[1234-5678,8100]
host1234
host1235
host1236
...
host5678
host8100
```

Provide the arguments on the command-line, you get out a newline-separated list
with each individual host.

Pre-empting some questions/feature requests:

- If you need the output separated by spaces or commas or whatever, use
  something like xargs
- It takes arguments, not standard input, suitable for pasting (although
  standard input would be nice later, for big lists)
- There's no way to reverse the format, turning lists into minimal expressions,
  but that sounds like a fun algorithmic challenge (I'm not sure why it would be
  useful)
- Every host has to have a number, because of the way that the parsing works, I
  guess just `cat` anything else on the end?
- Reverse ranges don't work, but I don't think Slurm or whatever would ever
  generate them anyway
- Some test cases would be nice
