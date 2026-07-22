# UploadPartUrl


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**part_number** | **int** |  | 
**upload_url** | **str** |  | 
**expires_at** | **str** |  | 

## Example

```python
from moqentra_client.models.upload_part_url import UploadPartUrl

# TODO update the JSON string below
json = "{}"
# create an instance of UploadPartUrl from a JSON string
upload_part_url_instance = UploadPartUrl.from_json(json)
# print the JSON string representation of the object
print(UploadPartUrl.to_json())

# convert the object into a dict
upload_part_url_dict = upload_part_url_instance.to_dict()
# create an instance of UploadPartUrl from a dict
upload_part_url_from_dict = UploadPartUrl.from_dict(upload_part_url_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


