
# TaskResponse


## Properties

Name | Type
------------ | -------------
`id` | string
`project_id` | string
`state` | string
`assignee` | string
`asset_ids` | Array&lt;string&gt;
`lease_expires_at` | string

## Example

```typescript
import type { TaskResponse } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "project_id": null,
  "state": null,
  "assignee": null,
  "asset_ids": null,
  "lease_expires_at": null,
} satisfies TaskResponse

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as TaskResponse
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


