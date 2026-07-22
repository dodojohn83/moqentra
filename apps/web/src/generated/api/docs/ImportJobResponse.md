
# ImportJobResponse


## Properties

Name | Type
------------ | -------------
`id` | string
`state` | string
`source_url` | string
`target_key` | string
`media_type` | string
`total_bytes` | number
`transferred_bytes` | number
`concurrency` | number
`deadline_seconds` | number
`digest` | string
`failure` | string
`retry_count` | number

## Example

```typescript
import type { ImportJobResponse } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "state": null,
  "source_url": null,
  "target_key": null,
  "media_type": null,
  "total_bytes": null,
  "transferred_bytes": null,
  "concurrency": null,
  "deadline_seconds": null,
  "digest": null,
  "failure": null,
  "retry_count": null,
} satisfies ImportJobResponse

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as ImportJobResponse
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


