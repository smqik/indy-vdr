package vdr

import (
	"github.com/google/uuid"
)

type GetContext struct {
	Operation `json:",inline"`
	Dest      string         `json:"dest"`
	Data      getContextData `json:"data"`
}

type getContextData struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

// type RichSchema struct {
// 	Operation `json:",inline"`
// 	Dest      string         `json:"dest"`
// 	Data      RichSchemaData `json:"data"`
// }

type Context struct {
	Operation `json:",inline"`
	Id        string      `json:"id"`
	Content   interface{} `json:"content"`
	RsName    string      `json:"rsName"`
	RsVersion string      `json:"rsVersion"`
	RsType    string      `json:"rsType"`
	Ver       string      `json:"ver"`
}

type ContextData struct {
	ID      string      `json:"id"`
	Name    string      `json:"rsName"`
	Version string      `json:"rsVersion"`
	Content interface{} `json:"content"`
	Type    string      `json:"rsType"`
}

func NewGetContext(issuerDID, name, version, from string) *Request {
	return &Request{
		Operation: GetContext{
			Operation: Operation{Type: GET_CONTEXT},
			Dest:      issuerDID,
			Data:      getContextData{Name: name, Version: version},
		},
		Identifier:      from,
		ProtocolVersion: 2,
		ReqID:           uuid.New().ID(),
	}
}

func NewContext(id, name, version, from string, content interface{}) *Request {
	return &Request{
		Operation: Context{
			Operation: Operation{Type: SET_CONTEXT},
			Id:        id,
			Content:   content,
			RsName:    name,
			RsVersion: version,
			RsType:    "ctx", // Assuming "sch" is the type for schema
			Ver:       "2",   // Assuming "2" is the version
		},
		Identifier:      from,
		ProtocolVersion: 2,
		ReqID:           uuid.New().ID(),
	}

	// return &Request{
	// 	Operation: RichSchema{
	// 		Operation: Operation{Type: SET_RICH_SCHEMA},
	// 		Dest:      issuerDID,
	// 		Data:      RichSchemaData{Name: name, Version: version, Content: content, Type: "sch"},
	// 	},
	// 	Identifier:      from,
	// 	ProtocolVersion: 2,
	// 	ReqID:           uuid.New().ID(),
	// }
}
