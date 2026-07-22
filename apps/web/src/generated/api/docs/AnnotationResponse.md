
# AnnotationResponse


## Properties

Name | Type
------------ | -------------
`id` | string
`task_id` | string
`asset_id` | string
`revision` | number
`client_update_id` | string
`actor_id` | string
`payload` | object

## Example

```typescript
import type { AnnotationResponse } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "task_id": null,
  "asset_id": null,
  "revision": null,
  "client_update_id": null,
  "actor_id": null,
  "payload": null,
} satisfies AnnotationResponse

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as AnnotationResponse
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


