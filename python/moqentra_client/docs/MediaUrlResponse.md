# MediaUrlResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**url** | **str** |  | 
**expires_at** | **str** |  | 

## Example

```python
from moqentra_client.models.media_url_response import MediaUrlResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MediaUrlResponse from a JSON string
media_url_response_instance = MediaUrlResponse.from_json(json)
# print the JSON string representation of the object
print(MediaUrlResponse.to_json())

# convert the object into a dict
media_url_response_dict = media_url_response_instance.to_dict()
# create an instance of MediaUrlResponse from a dict
media_url_response_from_dict = MediaUrlResponse.from_dict(media_url_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


