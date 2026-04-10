export type ModelQuality = 'Good' | 'Caution' | 'Not Recommended'

export const R2_GOOD_THRESHOLD = 0.8
const R2_WARN_THRESHOLD = 0.5

export function getModelQuality(rSquared: number): ModelQuality {
  if (rSquared >= R2_GOOD_THRESHOLD) return 'Good'
  if (rSquared >= R2_WARN_THRESHOLD) return 'Caution'
  return 'Not Recommended'
}
