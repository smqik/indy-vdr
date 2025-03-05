package vdr

import (
	"github.com/google/uuid"
)

type GetRichSchema struct {
	Operation `json:",inline"`
	Dest      string        `json:"dest"`
	Data      getSchemaData `json:"data"`
}

type getRichSchemaData struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

// type RichSchema struct {
// 	Operation `json:",inline"`
// 	Dest      string         `json:"dest"`
// 	Data      RichSchemaData `json:"data"`
// }

type RichSchema struct {
	Operation `json:",inline"`
	Id        string      `json:"id"`
	Content   interface{} `json:"content"`
	RsName    string      `json:"rsName"`
	RsVersion string      `json:"rsVersion"`
	RsType    string      `json:"rsType"`
	Ver       string      `json:"ver"`
}

type RichSchemaData struct {
	ID      string      `json:"id"`
	Name    string      `json:"rsName"`
	Version string      `json:"rsVersion"`
	Content interface{} `json:"content"`
	Type    string      `json:"rsType"`
}

func NewGetRichSchema(issuerDID, name, version, from string) *Request {
	return &Request{
		Operation: GetRichSchema{
			Operation: Operation{Type: GET_RICH_SCHEMA},
			Dest:      issuerDID,
			Data:      getSchemaData{Name: name, Version: version},
		},
		Identifier:      from,
		ProtocolVersion: 2,
		ReqID:           uuid.New().ID(),
	}
}

func NewRichSchema(issuerDID, name, version, from string, content interface{}) *Request {
	return &Request{
		Operation: RichSchema{
			Operation: Operation{Type: RICH_SCHEMA},
			Id:        issuerDID,
			Content:   content,
			RsName:    name,
			RsVersion: version,
			RsType:    "sch", // Assuming "sch" is the type for schema
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
