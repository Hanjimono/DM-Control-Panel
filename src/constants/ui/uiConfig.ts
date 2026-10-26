import type { UiConfig } from "@/ui/Config/uiConfig.types"

/**
 * Host overrides for pet-ui defaults.
 * Inputs use a label above a filled field, matching the UI kit.
 */
const localUiConfig: Partial<UiConfig> = {
  formInputVariant: "simple",
  modalHeaderTitleAs: "h3",
  tooltipArrowOptions: {
    fill: "#2F080D",
    stroke: "#2F080D",
    strokeWidth: 1,
    width: 14,
    height: 7
  }
}

export default localUiConfig
