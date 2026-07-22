# LabelUProjectConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**version** | **str** |  | 
**media_type** | **str** |  | 
**tools** | [**List[LabelUToolConfig]**](LabelUToolConfig.md) |  | 

## Example

```python
from moqentra_client.models.label_u_project_config import LabelUProjectConfig

# TODO update the JSON string below
json = "{}"
# create an instance of LabelUProjectConfig from a JSON string
label_u_project_config_instance = LabelUProjectConfig.from_json(json)
# print the JSON string representation of the object
print(LabelUProjectConfig.to_json())

# convert the object into a dict
label_u_project_config_dict = label_u_project_config_instance.to_dict()
# create an instance of LabelUProjectConfig from a dict
label_u_project_config_from_dict = LabelUProjectConfig.from_dict(label_u_project_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


