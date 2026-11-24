import {
  readOffice,
  validateOffice,
  renderOffice,
  recalculateWorkbook,
  convertOffice,
} from "./runtime.mjs";

const [action, input, ...args] = process.argv.slice(2);
try {
  if (!input)
    throw new Error(
      "Usage: cli.mjs read|validate|render|recalculate|convert <file> [options JSON or output path]",
    );
  const result =
    action === "read"
      ? await readOffice(input, args[0] ? JSON.parse(args[0]) : {})
      : action === "validate"
        ? await validateOffice(input)
        : action === "render"
          ? await renderOffice(input, args[0] ?? ".anya/office-preview")
          : action === "recalculate"
            ? await recalculateWorkbook(input, args[0] ?? ".anya/office-calculation")
            : action === "convert" && args[0]
              ? await convertOffice(input, args[0])
              : (() => {
                  throw new Error(`Unknown action: ${action}`);
                })();
  console.log(JSON.stringify(result, null, 2));
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
