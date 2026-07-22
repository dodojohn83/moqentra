# AddAssetRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** |  | 
**object_key** | **str** |  | 
**digest** | **str** |  | 
**size** | **int** |  | 
**media_type** | **str** |  | 
**metadata** | **object** |  | [optional] 

## Example

```python
from moqentra_client.models.add_asset_request import AddAssetRequest

# TODO update the JSON string below
json = "{}"
# create an instance of AddAssetRequest from a JSON string
add_asset_request_instance = AddAssetRequest.from_json(json)
# print the JSON string representation of the object
print(AddAssetRequest.to_json())

# convert the object into a dict
add_asset_request_dict = add_asset_request_instance.to_dict()
# create an instance of AddAssetRequest from a dict
add_asset_request_from_dict = AddAssetRequest.from_dict(add_asset_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


