# UploadSessionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**target_key** | **str** |  | 
**media_type** | **str** |  | 
**part_size** | **int** |  | 
**total_size** | **int** |  | 
**parts** | [**List[UploadPartInfo]**](UploadPartInfo.md) |  | 
**state** | **str** |  | 
**expires_at** | **str** |  | 

## Example

```python
from moqentra_client.models.upload_session_response import UploadSessionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of UploadSessionResponse from a JSON string
upload_session_response_instance = UploadSessionResponse.from_json(json)
# print the JSON string representation of the object
print(UploadSessionResponse.to_json())

# convert the object into a dict
upload_session_response_dict = upload_session_response_instance.to_dict()
# create an instance of UploadSessionResponse from a dict
upload_session_response_from_dict = UploadSessionResponse.from_dict(upload_session_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


