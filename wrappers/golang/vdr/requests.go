package vdr

import (
	"crypto/sha256"
	"encoding/json"
	"fmt"

	"github.com/google/uuid"
)

const (
	TRUSTEE         = "0"
	STEWARD         = "2"
	ENDORSER        = "101"
	NETWORK_MONITOR = "201"
)

const (
	NODE            = "0"
	NYM             = "1"
	ATTRIB          = "100"
	HANDLETXN       = "99994"
	GETHANDLETXN    = "99996"
	AUCTIONSTART    = "99990"
	SCHEMA          = "101"
	RICH_SCHEMA     = "201"
	SET_CONTEXT     = "200"
	CLAIM_DEF       = "102"
	POOL_UPGRADE    = "109"
	NODE_UPGRADE    = "110"
	POOL_CONFIG     = "111"
	GET_TXN         = "3"
	GET_ATTR        = "104"
	GET_NYM         = "105"
	GET_SCHEMA      = "107"
	GET_CLAIM_DEF   = "108"
	GET_AUTH_RULE   = "121"
	GET_RICH_SCHEMA = "301"
	GET_CONTEXT     = "300"
	protocolVersion = 2

	AuthActionAdd  = "ADD"
	AuthActionEdit = "EDIT"

	DefaultRequestDID = "LibindyDid111111111111"

	NoRole             = ""    // None (common USER)
	TrusteeRole        = "0"   // (TRUSTEE)
	Steward            = "2"   // (STEWARD)
	EndorserRole       = "101" // (ENDORSER)
	NetworkMonitorRole = "201" // (NETWORK_MONITOR))
)

type Request struct {
	Operation       interface{}    `json:"operation"`
	Identifier      string         `json:"identifier,omitempty"`
	Endorser        string         `json:"endorser,omitempty"`
	ReqID           uint32         `json:"reqId"`
	ProtocolVersion int            `json:"protocolVersion"`
	Signature       string         `json:"signature,omitempty"`
	TAAAcceptance   *TAAAcceptance `json:"taaAcceptance,omitempty"`
	Handle          string         `json:"handle,omitempty"`
}

type Operation struct {
	Type string `json:"type"`
	//Handle string                 `json:"handle,omitempty"`
	Dest string                 `json:"dest"`
	Data map[string]interface{} `json:"data,omitempty"`
}

type TAAAcceptance struct {
	Digest    string `json:"taaDigest,omitempty"`
	Mechanism string `json:"mechanism,omitempty"`
	Time      uint32 `json:"time,omitempty"`
}

type NymRequest struct {
	Operation `json:",inline"`
	Dest      string `json:"dest"`
}

type Nym struct {
	Operation     `json:",inline"`
	Dest          string `json:"dest"`
	Role          string `json:"role,omitempty"`
	DIDDocContent string `json:"diddocContent"`
	//	Handle        string `json:"handle"`
	Verkey string `json:"verkey,omitempty"`
}

type endpointvalue struct {
	Endpoint string `json:"endpoint"`
}

func NewNymRequest(did, from string) *Request {
	return &Request{
		Operation: NymRequest{
			Operation: Operation{Type: GET_NYM},
			Dest:      did,
		},
		Identifier:      from,
		ProtocolVersion: protocolVersion,
		ReqID:           uuid.New().ID(),
	}
}

func NewNym(did, verkey, from, role, diddoc string) *Request {
	return &Request{
		Operation: Nym{
			Operation:     Operation{Type: NYM},
			Dest:          did,
			Verkey:        verkey,
			DIDDocContent: diddoc,
			//Handle:        "myhandle",
			Role: role,
		},
		Identifier: from,
		//	Endorser:        from,
		ReqID:           uuid.New().ID(),
		ProtocolVersion: protocolVersion,
	}
}

type AttribRequest struct {
	Operation `json:",inline"`
	Dest      string `json:"dest"`
	Raw       string `json:"raw,omitempty"`
	Hash      string `json:"hash,omitempty"`
	Enc       string `json:"enc,omitempty"`
}

type HandleRequest struct {
	Operation `json:",inline"`
	Dest      string `json:"dest"`
	//Handle    string `json:"handle,omitempty"`
	Data map[string]interface{} `json:"-"`
}

type AuctionStartRequest struct {
	Operation `json:",inline"`
	Data      map[string]interface{} `json:"data,omitempty"`
	Dest      string                 `json:"dest"`
	// Handle    string `json:"handle,omitempty"`
}

type Attrib struct {
	Operation `json:",inline"`
	Dest      string                 `json:"dest"`
	Raw       interface{}            `json:"raw,omitempty"`
	Hash      string                 `json:"hash,omitempty"`
	Enc       string                 `json:"enc,omitempty"`
	Data      map[string]interface{} `json:"-"`
}

func NewRawAttribRequest(did, raw, from string) *Request {
	return newAttribRequest(AttribRequest{Operation: Operation{Type: GET_ATTR}, Dest: did, Raw: raw}, from)
}

func NewHandleRequest(did, from string, data map[string]interface{}) *Request {
	return newHandleRequest(HandleRequest{Operation: Operation{Type: HANDLETXN, Data: data, Dest: did}, Dest: did}, from)
}

func NewGetHandleRequest(did, from string, data map[string]interface{}) *Request {
	return newHandleRequest(HandleRequest{Operation: Operation{Type: GETHANDLETXN, Data: data, Dest: did}, Dest: did}, from)
}

func NewAuctionStartRequest(did, from string, data map[string]interface{}) *Request {
	return newAuctionStartRequest(AuctionStartRequest{Operation: Operation{Type: AUCTIONSTART, Dest: did}, Data: data, Dest: did}, from)
}

func NewHashAttribRequest(did, data, from string) *Request {
	hash := sha256.New().Sum([]byte(data))
	return newAttribRequest(AttribRequest{Operation: Operation{Type: GET_ATTR}, Dest: did, Hash: string(hash)}, from)
}

func NewEncAttribRequest(did, data, from string) *Request {
	enc := data //TODO, figure out how to encrypt
	return newAttribRequest(AttribRequest{Operation: Operation{Type: GET_ATTR}, Dest: did, Enc: enc}, from)
}

func newAttribRequest(attrReq AttribRequest, from string) *Request {
	return &Request{
		Operation:       attrReq,
		Identifier:      from,
		ProtocolVersion: protocolVersion,
		//	Handle:          "myhandle",
		ReqID: uuid.New().ID(),
	}
}

func newHandleRequest(handReq HandleRequest, from string) *Request {
	reqId := uuid.New().ID()
	fmt.Println("\n\nRequest ID is: ", reqId)
	return &Request{
		Operation:       handReq,
		Identifier:      from,
		ProtocolVersion: protocolVersion,
		//	Handle:          "handle",
		ReqID: reqId,
	}
}

func newGetHandleRequest(handReq HandleRequest, from string) *Request {
	reqId := uuid.New().ID()
	fmt.Println("\n\nRequest ID is: ", reqId)
	return &Request{
		Operation:       handReq,
		Identifier:      from,
		ProtocolVersion: protocolVersion,
		//	Handle:          "handle",
		ReqID: reqId,
	}
}

func newAuctionStartRequest(auctReq AuctionStartRequest, from string) *Request {
	reqId := uuid.New().ID()
	fmt.Println("\n\nRequest ID is: ", reqId)
	return &Request{
		Operation:       auctReq,
		Identifier:      from,
		ProtocolVersion: protocolVersion,
		Handle:          "myhandle",
		ReqID:           reqId,
	}
}

func NewRawAttrib(did, from string, data map[string]interface{}) *Request {
	d, _ := json.Marshal(data)
	return newAttrib(Attrib{Operation: Operation{Type: ATTRIB}, Dest: did, Raw: string(d)}, from)
}

func NewHashAttrib(did, data, from string) *Request {
	d, _ := json.Marshal(data)
	hash := sha256.New().Sum(d)
	return newAttrib(Attrib{Operation: Operation{Type: ATTRIB}, Dest: did, Hash: string(hash)}, from)
}

func NewEncAttrib(did, data, from string) *Request {
	//TODO: figure out how to enc
	enc := data
	return newAttrib(Attrib{Operation: Operation{Type: ATTRIB}, Dest: did, Enc: enc}, from)
}

func newAttrib(attrib Attrib, from string) *Request {
	return &Request{
		Operation:       attrib,
		Identifier:      from,
		ReqID:           uuid.New().ID(),
		ProtocolVersion: protocolVersion,
	}
}

type AuthRuleRequest struct {
	Operation  `json:",inline"`
	AuthAction string `json:"auth_action,omitempty"`
	AuthType   string `json:"auth_type,omitempty"`
	Field      string `json:"field,omitempty"`
}

type AuthAddRuleRequest struct {
	AuthRuleRequest
	NewValue string `json:"new_value"`
}

type AuthEditRuleRequest struct {
	AuthAddRuleRequest
	OldValue string `json:"old_value"`
}

func NewAuthRulesRequest() *Request {
	return newAuthRuleRequest(AuthRuleRequest{
		Operation: Operation{Type: GET_AUTH_RULE},
	})
}

func NewAuthAddRuleRequest(typ, field string) *Request {
	return newAuthRuleRequest(AuthAddRuleRequest{
		AuthRuleRequest: AuthRuleRequest{
			Operation:  Operation{Type: GET_AUTH_RULE},
			AuthType:   typ,
			AuthAction: AuthActionAdd,
			Field:      field,
		},
	})
}

func NewAuthEditRuleRequest(typ, field string) *Request {
	return newAuthRuleRequest(AuthEditRuleRequest{
		AuthAddRuleRequest: AuthAddRuleRequest{
			AuthRuleRequest: AuthRuleRequest{
				Operation:  Operation{Type: GET_AUTH_RULE},
				AuthType:   typ,
				AuthAction: AuthActionEdit,
				Field:      field,
			},
		},
	})
}

func newAuthRuleRequest(rule interface{}) *Request {
	return &Request{
		Operation:       rule,
		Identifier:      DefaultRequestDID,
		ProtocolVersion: protocolVersion,
		ReqID:           uuid.New().ID(),
	}
}
