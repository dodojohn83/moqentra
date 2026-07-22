
# CreateTrainingJobRequest


## Properties

Name | Type
------------ | -------------
`experiment_id` | string
`project_id` | string
`code_digest` | string
`image_digest` | string
`dataset_version_id` | string
`argv` | Array&lt;string&gt;

## Example

```typescript
import type { CreateTrainingJobRequest } from ''

// TODO: Update the object below with actual values
const example = {
  "experiment_id": null,
  "project_id": null,
  "code_digest": null,
  "image_digest": null,
  "dataset_version_id": null,
  "argv": null,
} satisfies CreateTrainingJobRequest

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as CreateTrainingJobRequest
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


