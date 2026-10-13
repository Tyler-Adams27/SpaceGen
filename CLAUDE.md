# SpaceGen

Desktop app that generates procedural space skyboxes. Read `CONTEXT.md` for the domain vocabulary, `docs/adr/` for architectural decisions and `docs/DECISIONS.md` for the running log of smaller decisions before changing anything.

## Rules

- **Decide autonomously and write the decision down.** The user delegated decisions from the MVP build onward. When a choice is not already settled in `CONTEXT.md`, an ADR or `docs/DECISIONS.md`, pick the best option, then append an entry to `docs/DECISIONS.md` with the choice, the alternatives and the reason. Ask the user only when blocked on something that cannot be decided from the repository or sensible defaults.
- **Work test-first, one red → green cycle at a time.** Write one failing test, run it and see it fail (red), write the minimum code to pass (green), then repeat. A test that starts green is proven by mutation: break the code on purpose, see the test fail, restore. Follow the `tdd` skill.
- **Test at public seams.** List the public interface and the seam under test before writing tests, and test behavior through that interface. GUI glue that cannot be unit-tested is kept thin and checked by running the app.
- **Use domain terms from `CONTEXT.md`** in names, tests and messages (Skybox, Layer, Seed, Project, Preview, Export, Face).
- **Record resolved terms in `CONTEXT.md` and hard-to-reverse trade-offs in `docs/adr/`** as they are decided.
- **Commit locally in small steps; never push.** End commit messages with the `Co-Authored-By` line given in the session's system reminder.
