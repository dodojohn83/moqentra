
# UploadSessionResponse


## Properties

Name | Type
------------ | -------------
`id` | string
`target_key` | string
`media_type` | string
`part_size` | number
`total_size` | number
`parts` | [Array&lt;UploadPartInfo&gt;](UploadPartInfo.md)
`state` | string
`expires_at` | string

## Example

```typescript
import type { UploadSessionResponse } from ''

// TODO: Update the object below with actual values
const example = {
  "id": null,
  "target_key": null,
  "media_type": null,
  "part_size": null,
  "total_size": null,
  "parts": null,
  "state": null,
  "expires_at": null,
} satisfies UploadSessionResponse

console.log(example)

// Convert the instance to a JSON string
const exampleJSON: string = JSON.stringify(example)
console.log(exampleJSON)

// Parse the JSON string back to an object
const exampleParsed = JSON.parse(exampleJSON) as UploadSessionResponse
console.log(exampleParsed)
```

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


