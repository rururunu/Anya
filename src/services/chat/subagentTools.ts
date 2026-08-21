/**
 * Tool names that spawn or encapsulate child agents.
 * Keep this list in one place so workbench, peek, and timeline UIs stay aligned
 * when new specialized agents are registered.
 */
export const SUBAGENT_TOOL_NAMES = [
  "run_subagent",
  "run_parallel_subagents",
  "run_skill",
  "explore_codebase",
  "research_topic",
  "review_code",
  "review_security",
  "generate_word",
  "docx",
  "pandoc",
] as const;

export type SubagentToolName = (typeof SUBAGENT_TOOL_NAMES)[number];

/** Fast membership check for sub-agent tool activity. */
export const SUBAGENT_TOOLS = new Set<string>(SUBAGENT_TOOL_NAMES);

/** Whether a tool name represents a sub-agent (or skill) invocation. */
export function isSubagentToolName(toolName: string): boolean {
  return SUBAGENT_TOOLS.has(toolName);
}
