# UploadPartInfo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**part_number** | **int** |  | 
**size** | **int** |  | 
**completed** | **bool** |  | 

## Example

```python
from moqentra_client.models.upload_part_info import UploadPartInfo

# TODO update the JSON string below
json = "{}"
# create an instance of UploadPartInfo from a JSON string
upload_part_info_instance = UploadPartInfo.from_json(json)
# print the JSON string representation of the object
print(UploadPartInfo.to_json())

# convert the object into a dict
upload_part_info_dict = upload_part_info_instance.to_dict()
# create an instance of UploadPartInfo from a dict
upload_part_info_from_dict = UploadPartInfo.from_dict(upload_part_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


