
# CreateUploadSessionRequest


## Properties

Name | Type
------------ | -------------
`resource_type` | string
`resource_id` | string
`version_id` | string
`name` | string
`media_type` | string
`part_size` | number
`total_size` | number
`ttl_seconds` | number

## Example

```typescript
import type { CreateUploadSessionRequest } from ''

// TODO: Update the object below with actual values
const example = {
  "resource_type": null,
  "resource_id": null,
  "version_id": null,
  "name": null,
  "media_type": null,
  "part_size": null,
  "total_size": null,
  "ttl_seconds": null,
} satisfies CreateUploadSessionRequest

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as CreateUploadSessionRequest
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


