# SpaceGen

Desktop app that generates procedural space skyboxes. Read `CONTEXT.md` for the domain vocabulary and `docs/adr/` for architectural decisions before changing anything.

## Rules

- **Ask the user at every decision.** When a choice is not already settled in `CONTEXT.md`, an ADR, or the user's message, ask and wait for the answer. Cover dependencies, crate and module layout, names, file formats, defaults, and tooling.
- **Work test-first, one red → green cycle at a time.** Write one failing test, run it and see it fail (red), write the minimum code to pass (green), then repeat. Write tests one at a time, each after the previous cycle's code. Follow the `tdd` skill.
- **Confirm seams before writing any test.** List the public interface and the seams under test, get the user's confirmation, then write tests only at confirmed seams.
- **Use domain terms from `CONTEXT.md`** in names, tests, and messages (Skybox, Layer, Seed, Project, Preview, Export, Face).
- **Record resolved terms in `CONTEXT.md` and hard-to-reverse trade-offs in `docs/adr/`** as they are decided.
