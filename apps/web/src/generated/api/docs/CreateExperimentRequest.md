
# CreateExperimentRequest


## Properties

Name | Type
------------ | -------------
`name` | string
`project_id` | string
`dataset_version_id` | string
`target_metric` | string

## Example

```typescript
import type { CreateExperimentRequest } from ''

// TODO: Update the object below with actual values
const example = {
  "name": null,
  "project_id": null,
  "dataset_version_id": null,
  "target_metric": null,
} satisfies CreateExperimentRequest

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as CreateExperimentRequest
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


