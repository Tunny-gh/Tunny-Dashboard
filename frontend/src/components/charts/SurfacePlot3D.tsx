import { useEffect, useState } from 'react'
import { DeckGL, GridLayer } from 'deck.gl'
import { useStudyStore } from '../../stores/studyStore'
import { useAnalysisStore } from '../../stores/analysisStore'
import { EmptyState } from '../common/EmptyState'
import type { SurrogateModelType } from '../../types'

// English comment.
const MODEL_OPTIONS: { value: SurrogateModelType; label: string; disabled?: boolean }[] = [
  { value: 'ridge', label: 'Ridge Regression' },
  { value: 'random_forest', label: 'Random Forest' },
  { value: 'kriging', label: 'Kriging (coming soon)', disabled: true },
]

export function SurfacePlot3D() {
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const {
    surrogateModelType,
    surface3dCache,
    isComputingSurface,
    surface3dError,
    setSurrogateModelType,
    computeSurface3d,
  } = useAnalysisStore()

  const paramNames = currentStudy?.paramNames ?? []
  const objectiveNames = currentStudy?.objectiveNames ?? []

  const [param1, setParam1] = useState<string>('')
  const [param2, setParam2] = useState<string>('')
  const [objective, setObjective] = useState<string>('')

  // English comment.
  useEffect(() => {
    if (paramNames.length >= 2) {
      setParam1((prev) => (prev && paramNames.includes(prev) ? prev : paramNames[0]))
      setParam2((prev) => {
        const fallback = paramNames.find((p) => p !== paramNames[0]) ?? paramNames[1]
        return prev && paramNames.includes(prev) && prev !== param1 ? prev : fallback
      })
    }
    if (objectiveNames.length >= 1) {
      setObjective((prev) => (prev && objectiveNames.includes(prev) ? prev : objectiveNames[0]))
    }
  }, [currentStudy]) // eslint-disable-line react-hooks/exhaustive-deps

  // English comment.
  useEffect(() => {
    if (!currentStudy || !param1 || !param2 || !objective || param1 === param2) return
    computeSurface3d(param1, param2, objective, 50)
  }, [currentStudy, param1, param2, objective, surrogateModelType, computeSurface3d])

  // English comment.
  if (!currentStudy) {
    return <EmptyState message="Please load data" />
  }

  if (paramNames.length < 2) {
    return <EmptyState message="At least 2 parameters required" />
  }

  if (isComputingSurface) {
    return <EmptyState message="Computing 3D surface..." />
  }

  if (surface3dError) {
    return <EmptyState message={`Surface computation error: ${surface3dError}`} />
  }

  // English comment.
  const cacheKey = `${surrogateModelType}_${param1}_${param2}_${objective}_50`
  const result = surface3dCache.get(cacheKey)

  // English comment.
  const gridData = result
    ? result.values.flatMap((row, i) =>
        row.map((val, j) => ({
          position: [result.grid1[j], result.grid2[i]] as [number, number],
          colorValue: val,
        })),
      )
    : []

  const layers = result
    ? [
        new GridLayer({
          id: 'surface3d-layer',
          data: gridData,
          getPosition: (d: { position: [number, number] }) => d.position,
          getColorWeight: (d: { colorValue: number }) => d.colorValue,
          colorAggregation: 'MEAN',
          cellSize: Math.abs((result.grid1[1] ?? 1) - (result.grid1[0] ?? 0)) * 1.05,
          extruded: true,
          pickable: true,
        }),
      ]
    : []

  return (
    <div
      data-testid="surface-plot-3d"
      style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '8px' }}
    >
      {/* Axis and model selection controls */}
      <div
        style={{
          display: 'flex',
          flexWrap: 'wrap',
          gap: '8px',
          marginBottom: '8px',
          fontSize: '12px',
        }}
      >
        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          X Axis:
          <select
            value={param1}
            onChange={(e) => setParam1(e.target.value)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {paramNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          Y Axis:
          <select
            value={param2}
            onChange={(e) => setParam2(e.target.value)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {paramNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          Objective:
          <select
            value={objective}
            onChange={(e) => setObjective(e.target.value)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {objectiveNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          Model:
          <select
            value={surrogateModelType}
            onChange={(e) => setSurrogateModelType(e.target.value as SurrogateModelType)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {MODEL_OPTIONS.map(({ value, label, disabled }) => (
              <option key={value} value={value} disabled={disabled}>
                {label}
              </option>
            ))}
          </select>
        </label>
      </div>

      {/* deck.gl 3D canvas */}
      <div style={{ flex: 1, position: 'relative' }}>
        {result ? (
          <DeckGL layers={layers} controller={true} style={{ width: '100%', height: '100%' }} />
        ) : (
          <EmptyState message="Select parameters to compute" />
        )}
      </div>
    </div>
  )
}
