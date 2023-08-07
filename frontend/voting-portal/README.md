# voting-portal
Voting portal for the next generation.

# 1. Compile the Ui library

The ballot verifier uses the common UI librarry [ui-essentials] as a github submodule.
If you're using devcontainers it should already be checked in, otherwise run:

```bash
git submodule update --init
```

Then you need to compile ui-essentials:

```bash
cd ui-essentials
yarn
yarn build
```

# 2. Use sequent-core

The ballot verifier uses [sequent-core] as a npm package. You need to compile it in another
place and then copy it to `rust/pkg/sequent-core-0.1.0.tgz `. Note that if its version
is changed you may need to update its hash in `voting-portal/yarn.lock` (use 
`sha1sum rust/pkg/sequent-core-0.1.0.tgz` to get the hash, or `shasum` instead of `sha1sum` if
you're in Mac Os X.

# 3. Run it

Just run `yarn` and then `yarn start`.
