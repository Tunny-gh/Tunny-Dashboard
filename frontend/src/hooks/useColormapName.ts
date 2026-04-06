import { useSelectionStore } from '../stores/selectionStore'
import type { ColormapName } from '../colormaps'

/** Returns the currently selected colormap name from the global selection store. */
export function useColormapName(): ColormapName {
  return useSelectionStore((s) => s.colorMode) as ColormapName
}
