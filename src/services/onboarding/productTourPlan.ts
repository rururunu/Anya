export type ProductTourStepId = "welcome" | "provider" | "composer" | "hotkey" | "workspace";

export interface ProductTourPlanStep {
  id: ProductTourStepId;
  element?: string;
  side?: "top" | "right" | "bottom" | "left";
  align?: "start" | "center" | "end";
  allowInteraction?: boolean;
}

/** 按当前是否已配置模型，排出引导步骤。缺少锚点的步骤由调用方滤掉。 */
export function planProductTour(hasProvider: boolean): ProductTourPlanStep[] {
  const steps: ProductTourPlanStep[] = [{ id: "welcome" }];
  if (!hasProvider) {
    steps.push({
      id: "provider",
      element: "[data-tour='provider-setup']",
      side: "right",
      align: "center",
      allowInteraction: true,
    });
  }
  steps.push(
    {
      id: "composer",
      element: "[data-tour='composer']",
      side: "top",
      align: "center",
      allowInteraction: true,
    },
    { id: "hotkey", element: "[data-tour='hotkey-hint']", side: "top", align: "center" },
    { id: "workspace", element: "[data-tour='workspace']", side: "right", align: "start" },
  );
  return steps;
}

/** 保留欢迎步，以及当前画面上找得到的锚点。 */
export function selectProductTourSteps(
  steps: ProductTourPlanStep[],
  isVisible: (selector: string) => boolean,
): ProductTourPlanStep[] {
  return steps.filter((step) => !step.element || isVisible(step.element));
}
