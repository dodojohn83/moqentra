
# CreateImportJobRequest


## Properties

Name | Type
------------ | -------------
`source_url` | string
`target_key` | string
`media_type` | string
`total_bytes` | number
`concurrency` | number
`deadline_seconds` | number

## Example

```typescript
import type { CreateImportJobRequest } from ''

// TODO: Update the object below with actual values
const example = {
  "source_url": null,
  "target_key": null,
  "media_type": null,
  "total_bytes": null,
  "concurrency": null,
  "deadline_seconds": null,
} satisfies CreateImportJobRequest

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as CreateImportJobRequest
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


