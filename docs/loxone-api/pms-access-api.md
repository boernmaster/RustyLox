# PMS & Access API

> **Version**: 1.0.1  
> **Source**: `pms-access-api.zip` (Loxone Swagger UI bundle)

The PMS & Access API is a robust solution designed to streamline many tasks within a hospitality management system.

 More details can be found in the User Configuration, Door Opening, Room Moods and Room Status sections below.

Some useful links:
- [User Management Documentation](https://www.loxone.com/wp-content/uploads/datasheets/UserManagement.pdf)
- [Structure File](https://www.loxone.com/wp-content/uploads/datasheets/StructureFile.pdf)
- [Communicating with the Loxone Miniserver](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf)
- [Loxone Home Page](https://www.loxone.com/)

## Base URL

```
https://miniserver-ip/jdev/sps
```

## Authentication

### `hospitality_auth`

- **Type**: `http`
- **Scheme**: `basic`

### `bearer_auth`

- **Type**: `http`
- **Scheme**: `bearer`
- **Bearer format**: `JWT`

---

## Endpoints

## User Configuration

User Configuration simplifies user creation and NFC tag assignment. These requests allow you to create a user, set the NFC code touch to learning mode, and assign the learned NFC tag to the user in a single command or just set the NFC code touch to learning mode and receive the NFC tag id.  

 An example use case would be the process of creating guest profiles and assigning learned NFC tags for room access in one seamless operation.

Each command uses the NFC code touch `{uuid}`. This can be found in the **LOXAPP** (StructureFile) which you can access ex.:
 *http://addressOfYourMiniserver/data/LoxApp3.json* 


By searching in *"controls"* for the name of your NFC code touch device you will find the *uuid* of that device. Example *uuid: 1e052949-03a5-4ef2-ffffbb105e80296f*
 An example of how the NFC Code Touch structure looks like in the Structure File can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=96).

**Note:** A system may have multiple NFC Code Touch devices. It is important that the user configures which NFC Code Touch device will be used for learning.  
  

**Important:** Assigning the user to a group is a crucial step. To do this, include the `{uuid}` of the desired groups in the *"usergroups"* field. For a list of available user groups and their corresponding `{uuid}`, refer to the GET */getgrouplist* section below.

A detailed description to the structures and their fields can be found at the bottom of the page under *Schemas*

More information on User Management functions and fields can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/UserManagement.pdf)

### `GET /getgrouplist`

**Lists all available user-groups and additional information**

Lists all available user-groups and additional information.

 **NOTE**: When configuring a new user, it is essential to assign a valid user group identified by `{uuid}` to the user being created.
 More information on User Management functions and fields can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/UserManagement.pdf)

**Responses:**

**`200`** — Successfully retrieved available user groups

```json
{
  "LL": {
    "value": "[{\"name\":\"User\",\"description\":\"User\",\"uuid\":\"1e05269f-015d-3de4-ffffbb105e80296f\",\"type\":0,\"userRights\":129},{\"name\":\"Full access\",\"description\":\"Full access\",\"uuid\":\"1e05269f-015d-3de8-ffffbb105e80296f\",\"type\":4,\"userRights\":4294967295}]\n",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message

> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `POST /configureuser`

**Create user and teach in NFC tag**

This operation creates or updates a user. 
* If `nfcUuid` is provided, the NFC Code Touch will be set to learning mode and waits the specified timeout until a tag is held to the NFC Code Touch and it has been successfully learnt. 
* If no `nfcUuid` is provided, only the user is created or updated without NFC assignment.
* User details must be passed as a JSON object in the request body. The JSON structure can be found under *Schemas->UserConfiguration* below or in the [documentation](https://www.loxone.com/wp-content/uploads/datasheets/UserManagement.pdf#page=15).
* **Important:** Assigning the user to a group is a crucial step. To do this, include the `{uuid}` of the desired groups in the *"usergroups"* field. For a list of available user groups and their corresponding `{uuid}`, refer to the GET */getgrouplist* section above.

The **password** as well as the **visualization password** for the user must be passed on as a **hash** in the JSON, below is how you can create a user password hash:

  - Use `jdev/sys/getkey2/{username}` to retrieve the `{salt}` and `{hashAlg}`.
    - salt is user-specific and long-lived.
    - **hashAlg** specifies which hashing algorithm to use (recent versions use **SHA256**).
    - A *"key"* property is also provided but is not used here; it is a temporarily valid key to be used for hashing when authenticating.
  - Use the `{hashAlg}` provided and the long-lived `{salt}` to create an uppercase `{passHash}` for the new `{password}`.
    - Example: `SHA256({password} + ":" + {salt}).toUpperCase() → {passHash}`

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `nfcUuid` | query | No | `string` | The uuid of the NFC code touch which will be set to learning mode. (example: `nfcUuid`) |
| `timeout` | query | No | `integer` | Value in milliseconds for how long the NFC code touch should stay in learning mode. If not passed on, default value is used. Default = 7000. (example: `7000`) |

**Request Body** (required):

Settings of user to create

Content-Type: `application/json`

Schema: [`UserConfiguration`](#userconfiguration)

**Responses:**

**`200`** — Successfully created user and assigned learned NFC tag

```json
{
  "LL": {
    "value": "{\"name\":\"Name\",\"desc\":\"\",\"uuid\":\"1e213699-03aa-168f-ffff504f94a1173b\",\"userid\":\"1236\",\"ocppid\":\"\",\"firstName\":\"\",\"lastName\":\"\",\"email\":\"\",\"phone\":\"\",\"uniqueUserId\":\"\",\"company\":\"\",\"department\":\"\",\"personalno\":\"\",\"title\":\"\",\"debitor\":\"\",\"customField1\":\"\",\"customField2\":\"\",\"customField3\":\"\",\"customField4\":\"\",\"customField5\":\"\",\"lastEdit\":505574388,\"userState\":0,\"isAdmin\":false,\"changePassword\":true,\"masterAdmin\":false,\"userRights\":16416,\"scorePWD\":0,\"scoreVisuPWD\":-1,\"trustMember\":\"\",\"usergroups\":[],\"nfcTags\":[{\"name\":\"Tester-1-editedNFC\",\"id\":\"7E D9 EA EC FE 2A 9F CB EC\"},{\"name\":\"NameNFC\",\"id\":\"2B 8F A3 AC 1A 6F E8 8A EC\"}],\"keycodes\":[{\"code\":\"A73289F7CAE7B78F9341026A482E578CD27D924F\"}],\"customFields\":[\"Custom Field 1\",\"Custom Field 2\",\"Custom Field 3\",\"Custom Field 4\",\"Custom Field 5\"]}\n",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message

```json
{
  "LL": {
    "value": "NFC learn failure",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /configureuser`

**Create user and teach in NFC tag**

This operation creates or updates a user. 
* If `nfcUuid` is provided, the NFC Code Touch will be set to learning mode and waits the specified timeout until a tag is held to the NFC Code Touch and it has been successfully learnt.
* If no `nfcUuid` is provided, only the user is created or updated without NFC assignment. 
* User details must be passed as a **Base64** encoding of the JSON string in the userjson query parameter. The JSON structure can be found under *Schemas->UserConfiguration* below or in the [documentation](https://www.loxone.com/wp-content/uploads/datasheets/UserManagement.pdf#page=15).
* It is **important** to assign the user to a group. You can do this by adding the `{uuid}` of the groups you want the user to be assigned to. To retrieve the available user groups and their `{uuid}` check out the GET */getgrouplist* section above.

The **password** as well as the **visualization password** for the user must be passed on as a **hash** in the JSON, below is how you can create a user password hash:

  - Use `jdev/sys/getkey2/{username}` to retrieve the `{salt}` and `{hashAlg}`.
    - salt is user-specific and long-lived.
    - **hashAlg** specifies which hashing algorithm to use (recent versions use **SHA256**).
    - A *"key"* property is also provided but is not used here; it is a temporarily valid key to be used for hashing when authenticating.
  - Use the `{hashAlg}` provided and the long-lived `{salt}` to create an uppercase `{passHash}` for the new `{password}`.
    - Example: `SHA256({password} + ":" + {salt}).toUpperCase() → {passHash}`


**NOTE:** We recommend you to use the **POST** */configureuser*
  method.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `nfcUuid` | query | No | `string` | The uuid of the NFC code touch which will be set to learning mode. (example: `nfcUuid`) |
| `timeout` | query | No | `integer` | Value in milliseconds for how long the NFC code touch should stay in learning mode. If not passed on, default value is used. Default = 7000. (example: `7000`) |
| `userjson` | query | Yes | `string` | The json with user parameters to create or edit. See UserConfiguration schema for structure.  **NOTE**: Use a **Base64** encoding for the user json. The user json example can be found under *Schemas->UserConfiguration* at the bottom of the page.  (example: `base64encodedjson`) |

**Responses:**

**`200`** — Successfully created user and assigned learned NFC tag

```json
{
  "LL": {
    "value": "{\"name\":\"Name\",\"desc\":\"\",\"uuid\":\"1e213699-03aa-168f-ffff504f94a1173b\",\"userid\":\"1236\",\"ocppid\":\"\",\"firstName\":\"\",\"lastName\":\"\",\"email\":\"\",\"phone\":\"\",\"uniqueUserId\":\"\",\"company\":\"\",\"department\":\"\",\"personalno\":\"\",\"title\":\"\",\"debitor\":\"\",\"customField1\":\"\",\"customField2\":\"\",\"customField3\":\"\",\"customField4\":\"\",\"customField5\":\"\",\"lastEdit\":505574388,\"userState\":0,\"isAdmin\":false,\"changePassword\":true,\"masterAdmin\":false,\"userRights\":16416,\"scorePWD\":0,\"scoreVisuPWD\":-1,\"trustMember\":\"\",\"usergroups\":[],\"nfcTags\":[{\"name\":\"Tester-1-editedNFC\",\"id\":\"7E D9 EA EC FE 2A 9F CB EC\"},{\"name\":\"NameNFC\",\"id\":\"2B 8F A3 AC 1A 6F E8 8A EC\"}],\"keycodes\":[{\"code\":\"A73289F7CAE7B78F9341026A482E578CD27D924F\"}],\"customFields\":[\"Custom Field 1\",\"Custom Field 2\",\"Custom Field 3\",\"Custom Field 4\",\"Custom Field 5\"]}\n",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message

```json
{
  "LL": {
    "value": "NFC learn failure",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /discovernfc/{nfcUuid}`

**Set NFC code touch to learning mode**

Sets the NFC code touch to learning mode. This mode is used to then *assign* an NFC tag to a *user*. Assigning the tag to a user is required to grant permissions to the tag, ensuring it can be used within the system.
* Results display NFC tag info. 
* You can specify the value in milliseconds for how long the NFC code touch should stay in learning mode. 
* If no timeout is provided, the default value of 7000 milliseconds is used.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `nfcUuid` | path | Yes | `string` | The uuid of the NFC code touch which will be set to learning mode. If this parameter is not passed on, then it will only create the user from the passed on body (example: `nfcUuid`) |
| `timeout` | query | No | `integer` | Value in milliseconds for how long the NFC code touch should stay in learning mode. If not passed on, default value is used. Default = 7000. (example: `7000`) |

**Responses:**

**`200`** — Successfully received NFC tag

```json
{
  "LL": {
    "value": "04 1A 63 BA 81 5E 80 00",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message

```json
{
  "LL": {
    "value": "NFC learn failure",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

## Door Opening

Door opening commands allow you to send a pulse to the outputs in the NFC code touch function block. 

 A typical usecase would be to open a door. 


To determine which *outputNr* should be triggered, you will need to find your NFC Code Touch in the **LOXAPP**(Structure File), for ex. using the name of the NFC code touch, and the *outputNr* can be found under "details"->"accessOutputs"->"q1","q2",...,"q6", where 1,2,..,6 are the available outputs. An example of how the NFC Code Touch structure looks like in the Structure File can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/StructureFile.pdf#page=97).


**Important note:** There are two get commands depending on the *User Interface* settings of your NFC code touch. 
To understand which of the two commands below to use, you will need to retrieve the *IsSecured* field from the NFC Code Touch you want to use from the **LOXAPP**(Structure File) which you can access ex.:
 *http://addressOfYourMiniserver/data/LoxApp3.json*
- If *IsSecured* is **false**, then the `/io/` get command is to be used
- If *IsSecured* is **true**, then the `/ios/` command is to be used. It is a secure command and requires an additional `{hash}` parameter. This is the hash value of the visualization password. How to properly create the hash can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14) under *General Info -> Secured Commands*

Each command uses the NFC code touch `{uuid}`. This can be found in the **LOXAPP** (StructureFile) which you can access ex.:
 *http://addressOfYourMiniserver/data/LoxApp3.json* 


 More info can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/StructureFile.pdf#page=98)

### `GET /io/{nfcUuid}/output/{outputNr}`

**Sends an impuls to the specific output on the NFC code touch block**

Sends an impuls to the specific output on the NFC code touch block, for example to open a door.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `nfcUuid` | path | Yes | `string` | The UUID of the NFC code touch on which the output will be triggered. (example: `nfcUuid`) |
| `outputNr` | path | Yes | `integer` | The output number to trigger. Output number can be 1-6. (example: `1`) |

**Responses:**

**`200`** — Successfully sent impuls to output.

```json
{
  "LL": {
    "value": "output/1",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message. Possible errors:
- 500: NFC code touch uses interface password
- 423: User is not permitted to execute the command at the moment / outputNr out of range


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{nfcUuid}/output/{outputNr}`

**Sends an impuls to the specific output on the NFC code touch block. Secure Command.**

Sends an impuls to the specific output on the NFC code touch block, for example to open a door.


**Process to create a hash for the visualization password:**
1. Request the visualization password from the user `{visuPw}`.
2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
   - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256, etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.
4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase `{visuPwHash}` and the `{key}` → `{hash}`.



More info on the hashing algorithm can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14) under *General Info -> Secured Commands*

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `nfcUuid` | path | Yes | `string` | The UUID of the NFC code touch on which the output will be triggered. (example: `nfcUuid`) |
| `outputNr` | path | Yes | `integer` | The output number to trigger. Output number can be 1-6. (example: `1`) |

**Responses:**

**`200`** — Successfully sent impuls to output.

```json
{
  "LL": {
    "value": "output/1",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message. Possible errors:
- 500: NFC code touch password hash incorrect
- 423: User is not permitted to execute the command at the moment / outputNr out of range


> **Auth**: `hospitality_auth`, `bearer_auth`

---

## Room Moods

Room Moods commands allow you to set a temperature in a room and which music to be played. This is done through the Intelligent Room Controller and Audio Player.

 **Setting Room Temperature:**
 Preferred room temperatures are preconfigured for each room, based on common comfort levels. The guest can choose their preferred room temperature to adjust the room to their liking. 

 For the commands you will need the `{ircUuid}` (Intelligent Room Controller UUID), this can be found in the **LOXAPP** (Structure File) which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. 
By searching in *"controls"* for the name of your Intelligent Room Controller you will find the *uuid* of that device. Example *uuid: 1e052949-03a5-4ef2-ffffbb105e80296f*
  An example of how the Intelligent Room Controller structure looks like in the Structure File can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=77). 

  It is **important** to know whether the Intelligent Room Controller is set to use a single temperature. This can be found in the **LOXAPP** (Structure File), for ex. using the name of the Intelligent Room Controller, and if it is using single temperature will be under "details"->"singleComfortTemperature".  
  - If this is set to `true`, then you should set only the comfort temperature `setComfortTemperature`.
  - If this is set to `false`, then `setComfortTemperature` sets the comfort temperature for heating and `setComfortTemperatureCool` sets the comfort temperature for cooling.
  
   An example of how the Intelligent Room Controller structure looks like in the Structure File can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=77).


**Setting Music Favorite:**
 Favorites are preconfigured in rooms, for example, using different music genres. Each favorite is assigned a numerical ID, and ideally, the same ID is used for each favorite across all rooms (e.g., through the function blocks template configurations). Using a specific command, a preferred genre or favorite can be preselected for a guest, which will then start playing first. The guest can still manually switch between the other available favorites afterward. 

 For the commands you will need the `{audioPlayerUuid}`, this can be found in the **LOXAPP** (Structure File) which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. 
By searching in *"controls"* for the name of your Audio Player you will find the *uuid* of that device. Example *uuid: 1e052949-03a5-4ef2-ffffbb105e80296f*
 An example of how the Audio Player structure looks like in the Structure File can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14). 

 Another parameter which you will need is the music *favorite ID*. The available music favorites: their priority in the list, numerical ID and name can be retrieved with the *GET /getmusicfavorites* commands below.

**Important note:** There are different get commands depending on the *User Interface* settings of your Intelligent Room Controller and Audio Player. 
To understand which of the commands below to use, you will need to retrieve the *IsSecured* field from the Intelligent Room Controller or Audio Player you want to use from the **LOXAPP**(Structure File) which you can access ex.:
 *http://addressOfYourMiniserver/data/LoxApp3.json*
- If *IsSecured* is **false**, then the `/io/` get command is to be used
- If *IsSecured* is **true**, then the `/ios/` command is to be used. It is a secure command and requires an additional `{hash}` parameter. This is the hash value of the visualization password. How to properly create the hash can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14) under *General Info -> Secured Commands* 


More info can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/StructureFile.pdf#page=82)

### `GET /io/{ircUuid}/setComfortTemperature/{temp}`

**Sets the temperature on the Intelligent Room Controller.**

Sets the comfort temperature on the Intelligent Room Controller. 
**NOTE:** If the block doesn't use a single temperature, then this sets the comfort temperature for heating.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `ircUuid` | path | Yes | `string` | The UUID of the Intelligent Room Controller on which the temperature will be set. (example: `ircUuid`) |
| `temp` | path | Yes | `number` | The temperature to be set. (example: `22.5`) |

**Responses:**

**`200`** — Successfully set temperature.

```json
{
  "LL": {
    "value": "setcomforttemperature/20",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message. Possible errors:
- 500: Intelligent Room Controller uses secure command
- 423: User is not permitted to execute the command at the moment


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /io/{ircUuid}/setComfortTemperatureCool/{temp}`

**Sets the cooling comfort temperature on the Intelligent Room Controller.**

Sets the cooling comfort temperature on the Intelligent Room Controller. 

**NOTE:** if the block doesn't use a single temperature, then this command is to be used. If it does use a single temperature, then the `/setComfortTemperature` command is to be used.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `ircUuid` | path | Yes | `string` | The UUID of the Intelligent Room Controller on which the temperature will be set. (example: `ircUuid`) |
| `temp` | path | Yes | `number` | The temperature to be set. (example: `14`) |

**Responses:**

**`200`** — Successfully set temperature.

```json
{
  "LL": {
    "value": "setcomforttemperaturecool/14",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message. Possible errors:
- 500: Intelligent Room Controller uses secure command
- 423: User is not permitted to execute the command at the moment


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{ircUuid}/setComfortTemperature/{temp}`

**Sets the temperature on the Intelligent Room Controller. Secure Command.**

Sets the comfort temperature on the Intelligent Room Controller. 
This is a secure command and requires a hash value of the visualization password. 

**Process to create a hash for the visualization password:**
1. Request the visualization password from the user `{visuPw}`.
2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
    - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256, etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.
4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase `{visuPwHash}` and the `{key}` → `{hash}`.



More info on the hashing algorithm can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14) under *General Info -> Secured Commands*
**NOTE:** If the block doesn't use a single temperature, then this sets the comfort temperature for heating.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `ircUuid` | path | Yes | `string` | The UUID of the Intelligent Room Controller on which the temperature will be set. (example: `ircUuid`) |
| `temp` | path | Yes | `number` | The temperature to be set. (example: `22.5`) |

**Responses:**

**`200`** — Successfully set temperature.

```json
{
  "LL": {
    "value": "setcomforttemperature/20",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message. Possible errors:
- 500: Intelligent Room Controller visualization password hash incorrect
- 423: User is not permitted to execute the command at the moment


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{ircUuid}/setComfortTemperatureCool/{temp}`

**Sets the cooling comfort temperature on the Intelligent Room Controller. Secure Command.**

Sets the cooling comfort temperature on the Intelligent Room Controller. 
This is a secure command and requires a hash value of the visualization password. 

**Process to create a hash for the visualization password:**
1. Request the visualization password from the user `{visuPw}`.
2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
    - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256, etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.
4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase `{visuPwHash}` and the `{key}` → `{hash}`.



More info on the hashing algorithm can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14) under *General Info -> Secured Commands*
**NOTE:** If the block doesn't use a single temperature, then this command is to be used. If it does use a single temperature, then the `/setComfortTemperature` secure command is to be used.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `ircUuid` | path | Yes | `string` | The UUID of the Intelligent Room Controller on which the temperature will be set. (example: `ircUuid`) |
| `temp` | path | Yes | `number` | The temperature to be set. (example: `14`) |

**Responses:**

**`200`** — Successfully set temperature.

```json
{
  "LL": {
    "value": "setcomforttemperaturecool/14",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message. Possible errors:
- 500: Intelligent Room Controller visualization password hash incorrect
- 423: User is not permitted to execute the command at the moment


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /io/{audioPlayerUuid}/getmusicfavorites`

**Retrieves the music favorites.**

Retrieves the music favorites.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `audioPlayerUuid` | path | Yes | `string` | The UUID of the Audio Player on which the first music favorite will be set. (example: `audioPlayerUuid`) |

**Responses:**

**`200`** — Successfully retrieved music favorites.

```json
{
  "LL": {
    "value": "{ \"favorites\": [{ \"priority\": 1, \"id\": 3, \"name\": \"Rock\" }, { \"priority\": 2, \"id\": 1, \"name\": \"Jazz\" }, { \"priority\": 3, \"id\": 4, \"name\": \"Pop\" }, { \"priority\": 4, \"id\": 5, \"name\": \"Hip-Hop\" }, { \"priority\": 5, \"id\": 2, \"name\": \"Classical\" }] }\n",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 400: Audio Player has no favorites
 - 404: no Audio Player with given audioPlayerUuid found
 - 500: Audio Player uses visualization password
 


```json
{
  "LL": {
    "value": "no music favorites",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{audioPlayerUuid}/getmusicfavorites`

**Retrieves the music favorites.**

Retrieves the music favorites. 
This is a secure command and requires a hash value of the
visualization password. 

**Process to create a hash for the
visualization password:**

1. Request the visualization password from the user `{visuPw}`.

2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from
the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
    - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256,
etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.

4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase
`{visuPwHash}` and the `{key}` → `{hash}`.




More info on the hashing algorithm can be found
[here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14)
under *General Info -> Secured Commands*

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `audioPlayerUuid` | path | Yes | `string` | The UUID of the Audio Player on which the first music favorite will be set. (example: `audioPlayerUuid`) |

**Responses:**

**`200`** — Successfully retrieved music favorites.

```json
{
  "LL": {
    "value": "{ \"favorites\": [{ \"priority\": 1, \"id\": 3, \"name\": \"Rock\" }, { \"priority\": 2, \"id\": 1, \"name\": \"Jazz\" }, { \"priority\": 3, \"id\": 4, \"name\": \"Pop\" }, { \"priority\": 4, \"id\": 5, \"name\": \"Hip-Hop\" }, { \"priority\": 5, \"id\": 2, \"name\": \"Classical\" }] }\n",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 400: Audio Player has no favorites
 - 404: no Audio Player with given audioPlayerUuid found
 - 500: Audio Player visualization password incorrect
 


```json
{
  "LL": {
    "value": "no music favorites",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /io/{audioPlayerUuid}/setmusicfavorite/{favoriteID}`

**Sets the default played room music favorite.**

Sets the default played room music favorite.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `audioPlayerUuid` | path | Yes | `string` | The UUID of the Audio Player on which the first music favorite will be set. (example: `audioPlayerUuid`) |
| `favoriteID` | path | Yes | `number` | The ID of the music favorite to be set as first choice. (example: `1`) |

**Responses:**

**`200`** — Successfully set music favorite.

```json
{
  "LL": {
    "value": 1,
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 404: no Audio Player with given audioPlayerUuid found
 - 500: Audio Player uses visualization password
 


```json
{
  "LL": {
    "value": "favorite id value is invalid",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{audioPlayerUuid}/setmusicfavorite/{favoriteID}`

**Sets the default played room music favorite. Secure Command.**

Sets the default played room music favorite. 
This is a secure command and requires a hash value of the
visualization password. 

**Process to create a hash for the
visualization password:**

1. Request the visualization password from the user `{visuPw}`.

2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from
the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
    - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256,
etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.

4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase
`{visuPwHash}` and the `{key}` → `{hash}`.




More info on the hashing algorithm can be found
[here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14)
under *General Info -> Secured Commands*

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `audioPlayerUuid` | path | Yes | `string` | The UUID of the Audio Player on which the first music favorite will be set. (example: `audioPlayerUuid`) |
| `favoriteID` | path | Yes | `number` | The ID of the music favorite to be set as first choice. (example: `1`) |

**Responses:**

**`200`** — Successfully set music favorite.

```json
{
  "LL": {
    "value": 1,
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 404: no Audio Player with given audioPlayerUuid found
 - 500: Audio Player visualization password hash incorrect
 


```json
{
  "LL": {
    "value": "favorite id value is invalid",
    "Code": "200"
  }
}
```

> **Auth**: `hospitality_auth`, `bearer_auth`

---

## Room Status

Room Status commands allow you to retrieve the current room status or activate an existing status. 

 An example use case would be checking the status of a room. For instance, if the room status is unclean, then a worker can be sent to clean it and then you can update the status to clean. Similarly, you could check if the room is currently occupied or has a "do not disturb" status set by the guest. 

 For the commands you will need the `{roomstatusUuid}` (Room Status Function Block UUID), this can be found in the **LOXAPP** (Structure File) which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. 
By searching in *"controls"* for the name of your Room Status function block you will find the *uuid* of that device. Example *uuid: 1e052949-03a5-4ef2-ffffbb105e80296f*
 Here you can also find the mappings of the ids to the status outputs in the field *details->outputs*.
  An example of how the Room Status block structure looks like in the Structure File can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=120) (Section Radio).

**Important note:** There are different get commands depending on the *User Interface* settings of your Room Status block. 
To understand which of the commands below to use, you will need to retrieve the *IsSecured* field from the Room Status block you want to use from the **LOXAPP**(Structure File) which you can access ex.:
 *http://addressOfYourMiniserver/data/LoxApp3.json*
- If *IsSecured* is **false**, then the `/io/` get command is to be used
- If *IsSecured* is **true**, then the `/ios/` command is to be used. It is a secure command and requires an additional `{hash}` parameter. This is the hash value of the visualization password. How to properly create the hash can be found [here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14) under *General Info -> Secured Commands*

### `GET /io/{roomstatusUuid}/status`

**Retrieves the current Room Status id.**

Retrieves the current Room Status id. 


The mappings of the ids to the status outputs can be found in the **LOXAPP** (Structure File), which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. By searching in "controls" for the name of your Room Status block you will find its properties and in the field *details->outputs* will be the status id mappings.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `roomstatusUuid` | path | Yes | `string` | The UUID of the Room Status block from which the status should be retrieved. (example: `roomstatusUuid`) |

**Responses:**

**`200`** — Successfully retrieved current room status.<br>
**Note:** In case "value": "0", this means no output currently active.


```json
{
  "LL": {
    "value": 1,
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 404: no Room Status block with given roomstatusUuid found
 - 500: Room Status block uses visualization password
 


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{roomstatusUuid}/status`

**Retrieves the current Room Status id. Secure Command.**

Retrieves the current Room Status id.

 The mappings of the ids to the status outputs can be found in the **LOXAPP** (Structure File), which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. By searching in "controls" for the name of your Room Status block you will find its properties and in the field *details->outputs* will be the status id mappings. 

This is a secure command and requires a hash value of the
visualization password. 

**Process to create a hash for the
visualization password:**

1. Request the visualization password from the user `{visuPw}`.

2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from
the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
    - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256,
etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.

4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase
`{visuPwHash}` and the `{key}` → `{hash}`.




More info on the hashing algorithm can be found
[here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14)
under *General Info -> Secured Commands*

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `roomstatusUuid` | path | Yes | `string` | The UUID of the Audio Player on which the first music favorite will be set. (example: `roomstatusUuid`) |

**Responses:**

**`200`** — Successfully retrieved current room status.<br>
**Note:** In case "value": "0", this means no output currently active.


```json
{
  "LL": {
    "value": 1,
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 404: no Room Status block with given roomstatusUuid found
 - 500: Room Status block visualization password hash incorrect
 


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /io/{roomstatusUuid}/{idToActivate}`

**Sets a Room Status from the given id.**

Sets the Room Status to the given id. 


The mappings of the ids to the status outputs can be found in the **LOXAPP** (Structure File), which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. By searching in "controls" for the name of your Room Status block you will find its properties and in the field *details->outputs* will be the status id mappings.

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `roomstatusUuid` | path | Yes | `string` | The UUID of the Room Status block from which the status should be retrieved. (example: `roomstatusUuid`) |
| `idToActivate` | path | Yes | `integer` | The status id to be set. (example: `1`) |

**Responses:**

**`200`** — Successfully set room status.


```json
{
  "LL": {
    "value": "1",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 404: no Room Status block with given roomstatusUuid found
 - 500: Room Status block uses visualization password ("value": "")
 - 500: Invalid status id (ex.: "value":"17")
 


> **Auth**: `hospitality_auth`, `bearer_auth`

---

### `GET /ios/{hash}/{roomstatusUuid}/{idToActivate}`

**Sets a Room Status from the given id. Secure Command.**

Sets the Room Status to the given id.

 The mappings of the ids to the status outputs can be found in the **LOXAPP** (Structure File), which you can access ex.:*http://addressOfYourMiniserver/data/LoxApp3.json*. By searching in "controls" for the name of your Room Status block you will find its properties and in the field *details->outputs* will be the status id mappings. 

This is a secure command and requires a hash value of the
visualization password. 

**Process to create a hash for the
visualization password:**

1. Request the visualization password from the user `{visuPw}`.

2. Request the `key`, `salt`, and the hashing algorithm `{hashAlg}` from
the Miniserver using the endpoint: `jdev/sys/getvisusalt/{user}`.
    - `{user}`: The user whose visualization password has been entered.
3. Create a hash using the specified `hashAlg` (e.g., SHA1, SHA256,
etc.) of the format: `{visuPw}:{salt}` → `{visuPwHash}`.

4. Generate an HMAC-SHA1 or HMAC-SHA256 hash using the uppercase
`{visuPwHash}` and the `{key}` → `{hash}`.




More info on the hashing algorithm can be found
[here](https://www.loxone.com/wp-content/uploads/datasheets/CommunicatingWithMiniserver.pdf#page=14)
under *General Info -> Secured Commands*

**Query Parameters:**

| Name | In | Required | Type | Description |
|------|----|----------|------|-------------|
| `hash` | path | Yes | `string` | Hash of the visualization password (example: `hash`) |
| `roomstatusUuid` | path | Yes | `string` | The UUID of the Audio Player on which the first music favorite will be set. (example: `roomstatusUuid`) |
| `idToActivate` | path | Yes | `integer` | The status id to be set. (example: `1`) |

**Responses:**

**`200`** — Successfully set room status.


```json
{
  "LL": {
    "value": "1",
    "Code": "200"
  }
}
```

**`default`** — Error response with code and message.
 - 404: no Room Status block with given roomstatusUuid found
 - 500: Room Status block visualization password hash incorrect ("value": "")
 - 500: Invalid status id (ex.: "value":"17")
 


> **Auth**: `hospitality_auth`, `bearer_auth`

---

## Schemas

### `UserConfiguration` {#userconfiguration}

| Field | Type | Required | Description | Example |
|-------|------|----------|-------------|---------|
| `name` | `string` | Yes | When it comes to users, this is the username that is used to login via our app. | `` |
| `userid` | `string` | No | May be empty, this is the id that will be returned by the NFC permission block when granting access. In Loxone Config, this field is configured as NFC Code Touch ID | `` |
| `isAdmin` | `boolean` | No | Indicates if the user has administrative rights on the Miniserver. | `` |
| `changePassword` | `boolean` | No | Specifies whether or not a user is allowed to change its passwords from within the apps. | `` |
| `masterAdmin` | `boolean` | No | In config versions prior to 11.0, there used to be one main admin, which could not be removed. | `` |
| `userRights` | `integer` | No | The rights or permissions associated with the user. | `` |
| `password` | `string` | No | Hash of the user password to be set. More details on how to set create the hash can be found in the User Configuration section. | `` |
| `visupassword` | `string` | No | Hash of the user visualization password to be set. More details on how to set create the hash can be found in the User Configuration section. | `` |
| `scorePWD` | `integer` | No | Provides/sets info on how strong a password is | `` |
| `scoreVisuPWD` | `integer` | No | Same like scorePWD but for visualization passwords. (additional password that has to be entered, even tough the connection itself is already authenticated - e.g. for disarming a burglar alarm). | `` |
| `userState` | `integer` | No | Indicates whether or not a user is active and may log in or get access (depending on the rights granted in config permission management).  | `` |
| `usergroups` | `array of string` | No | An array containing an object for each group the user should be part of. Each group object contains the UUID of the group. | `` |
| `nfcTags` | `array of string` | No | An array with an entry for each NFC tag associated with this user. Each tag is represented by a name and the NFC tag id | `` |
| `keycodes` | `array of string` | No | Even though this is an array of string codes, currently there is only one keycode for each user. The only attribute of each keycode object is the code itself. It should be a numeric code (0-9) with 2-8 digits passed on as a string, which will be hashed once stored. | `` |
